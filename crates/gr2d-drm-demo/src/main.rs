#![feature(cstr_display)]

mod app;
mod drm_util;

use std::{
    collections::{HashMap, VecDeque},
    num::NonZeroU32,
    os::fd::AsFd,
};

use colorspace::{Argb8888, Format};
use drm::{
    Device as _,
    buffer::DrmFourcc,
    control::{
        self, AtomicCommitFlags, Device as _, ResourceHandles, atomic::AtomicModeReq, connector,
        crtc, encoder, framebuffer, plane, property,
    },
};
use futures_util::StreamExt;
use host1x_drm::{
    buffer::PixelBuffer,
    device::TegraDRM,
    fence::Syncobj,
    uapi::{
        v1::buffer::Buffer,
        v2::{channel::Channel, queue::Queue, syncpt::SyncPoint},
    },
};
use tokio::{
    io::{self, unix::AsyncFd},
    join,
};
use tracing::field;

use crate::{app::App, drm_util::DrmEventStream};

#[macro_use]
extern crate tracing;

#[tokio::main]
async fn main() {
    let _guard = instrumentation::init();

    let gpu = TegraDRM::open("/dev/dri/card1").unwrap();
    gpu.set_client_capability(drm::ClientCapability::UniversalPlanes, true)
        .unwrap();
    gpu.set_client_capability(drm::ClientCapability::Atomic, true)
        .unwrap();

    let sync = SyncPoint::alloc(&gpu).unwrap();
    let queue: Queue<gr2d_reg::Client> = Queue::new(Channel::new(&gpu).unwrap(), sync);

    // FIXME: some of DRM code may be blocking.
    let res = gpu.resource_handles().unwrap();
    let encoders = get_encoders(&gpu, &res);
    let crtcs = get_crtcs(&gpu, &res);

    let hwplanes: Vec<_> = gpu
        .plane_handles()
        .unwrap()
        .iter()
        .map(|&handle| gpu.get_plane(handle).unwrap())
        .collect();

    let connector = res
        .connectors()
        .iter()
        .map(|&con| gpu.get_connector(con, true).unwrap())
        .filter(|con| con.state() == connector::State::Connected)
        .next()
        .expect("no active connector");
    let connector_props = gpu
        .get_properties(connector.handle())
        .expect("can't get connector properties")
        .as_hashmap(&gpu)
        .expect("can't get connector properties");

    info!(
        "using {:?}-{}",
        connector.interface(),
        connector.interface_id()
    );
    let mode = select_mode(connector.modes()).expect("can't select video mode");
    info!("using mode {}", mode.name().display());
    let crtc =
        select_crtc_encoder(&res, &crtcs, &encoders, &connector).expect("can't find suitable crtc");
    let crtc_props = gpu
        .get_properties(crtc.handle())
        .expect("can't get crtc properies")
        .as_hashmap(&gpu)
        .expect("can't get crtc properies");
    let primary_plane = select_plane(&hwplanes, Argb8888::DRM_FCC).unwrap();
    let primary_plane_props = gpu
        .get_properties(primary_plane.handle())
        .expect("can't get plane properties")
        .as_hashmap(&gpu)
        .expect("can't get plane properties");

    let mut buf0 = Buffer::<PixelBuffer<Argb8888>>::new(
        &gpu,
        NonZeroU32::new(mode.size().0 as _).unwrap(),
        NonZeroU32::new(mode.size().1 as _).unwrap(),
    )
    .unwrap()
    .mmap_rw()
    .unwrap();
    let fb0 = gpu.add_framebuffer(&buf0, 24, 32).unwrap();
    let buf0 = queue.channel().map_buffer(&mut buf0).unwrap();

    let mut buf1 = Buffer::<PixelBuffer<Argb8888>>::new(
        &gpu,
        NonZeroU32::new(mode.size().0 as _).unwrap(),
        NonZeroU32::new(mode.size().1 as _).unwrap(),
    )
    .unwrap()
    .mmap_rw()
    .unwrap();
    let fb1 = gpu.add_framebuffer(&buf1, 24, 32).unwrap();
    let buf1 = queue.channel().map_buffer(&mut buf1).unwrap();

    let mut swapchain = VecDeque::with_capacity(2);
    swapchain.push_back((fb0, buf0));
    swapchain.push_back((fb1, buf1));

    let drm_afd = AsyncFd::new(gpu.as_fd()).unwrap();
    let mut drm_event_stream = DrmEventStream::new(drm_afd);

    let app = App::new();
    loop {
        let mut fence = Syncobj::new(&gpu).unwrap();
        let (fb, buf) = swapchain.pop_front().unwrap();
        let task = app.render(&queue, &mut fence, buf);
        let wait_page_flip = async {
            while let Some(event) = drm_event_stream.next().await.transpose()? {
                match event {
                    control::Event::PageFlip(event) if event.crtc == crtc.handle() => {
                        return Ok(Some(event));
                    }
                    _ => (),
                }
            }

            Result::<_, io::Error>::Ok(None)
        };

        let mut atomic_req = AtomicModeReq::new();
        atomic_add_con(
            &gpu,
            &mut atomic_req,
            &connector,
            &connector_props,
            &crtc,
            &crtc_props,
            &mode,
            &[(
                &PlaneConfig {
                    plane: primary_plane.handle(),
                    fb: Some(fb),
                    src_x: 0,
                    src_y: 0,
                    src_w: mode.size().0,
                    src_h: mode.size().1,
                    crtc_x: 0,
                    crtc_y: 0,
                    crtc_w: mode.size().0,
                    crtc_h: mode.size().1,
                },
                &primary_plane_props,
            )],
        );
        gpu.atomic_commit(
            AtomicCommitFlags::ALLOW_MODESET
                | AtomicCommitFlags::NONBLOCK
                | AtomicCommitFlags::PAGE_FLIP_EVENT,
            atomic_req,
        )
        .expect("atomic commit fail");

        let (buf, flip) = join!(task, wait_page_flip);
        let flip = flip.unwrap().unwrap();
        trace!("flip duration: {} ms", flip.duration.as_millis());
        swapchain.push_back((fb, buf));
    }
}

fn get_encoders(gpu: &TegraDRM, res: &ResourceHandles) -> HashMap<encoder::Handle, encoder::Info> {
    res.encoders()
        .iter()
        .flat_map(|&handle| {
            gpu.get_encoder(handle).inspect_err(|error| {
                error!(
                    error = field::display(error),
                    handle = u32::from(handle),
                    "can't get encoder"
                )
            })
        })
        .map(|info| (info.handle(), info))
        .collect()
}

fn get_crtcs(gpu: &TegraDRM, res: &ResourceHandles) -> HashMap<crtc::Handle, crtc::Info> {
    res.crtcs()
        .iter()
        .flat_map(|&handle| {
            gpu.get_crtc(handle).inspect_err(|error| {
                error!(
                    error = field::display(error),
                    handle = u32::from(handle),
                    "can't get crtc"
                )
            })
        })
        .map(|info| (info.handle(), info))
        .collect()
}

fn select_mode(modes: &[control::Mode]) -> Option<control::Mode> {
    if let Some(&mode) = modes
        .iter()
        .find(|m| m.mode_type().intersects(control::ModeTypeFlags::PREFERRED))
    {
        return Some(mode);
    }

    modes.get(0).copied()
}

fn select_crtc_encoder(
    res: &ResourceHandles,
    crtcs: &HashMap<crtc::Handle, crtc::Info>,
    encoders: &HashMap<encoder::Handle, encoder::Info>,
    con: &connector::Info,
) -> Option<crtc::Info> {
    // If crtc is used already, keep using that. Often works much faster than when switching.
    if let Some(handle) = con.current_encoder() {
        if let Some(encoder) = encoders.get(&handle) {
            if let Some(handle) = encoder.crtc() {
                if let Some(&crtc) = crtcs.get(&handle) {
                    info!(
                        "keeping crtc {} encoder {} ({:?})",
                        u32::from(crtc.handle()),
                        u32::from(encoder.handle()),
                        encoder.kind()
                    );
                    return Some(crtc);
                } else {
                    warn!("crtc {} not found, ignoring", u32::from(handle));
                }
            }
        } else {
            warn!("encoder {} not found, ignoring", u32::from(handle));
        }
    }

    for &handle in con.encoders() {
        let Some(encoder) = encoders.get(&handle) else {
            warn!("encoder {} not found, ignoring", u32::from(handle));
            continue;
        };
        let supported_crtc_handles = res.filter_crtcs(encoder.possible_crtcs());
        for handle in supported_crtc_handles {
            let Some(&crtc) = crtcs.get(&handle) else {
                warn!("crtc {} not found, ignoring", u32::from(handle));
                continue;
            };
            if crtc.mode().is_some() {
                warn!("crtc {} already used, ignoring", u32::from(crtc.handle()));
            }
            info!(
                "using crtc {} encoder {} ({:?})",
                u32::from(crtc.handle()),
                u32::from(encoder.handle()),
                encoder.kind()
            );
            return Some(crtc);
        }
    }

    None
}

fn select_plane(planes: &[plane::Info], format: DrmFourcc) -> Option<plane::Info> {
    planes
        .iter()
        .find(|plane| plane.formats().iter().any(|&f| f == format as u32))
        .cloned()
}

struct PlaneConfig {
    plane: plane::Handle,
    fb: Option<framebuffer::Handle>,
    src_x: u16,
    src_y: u16,
    src_w: u16,
    src_h: u16,
    crtc_x: i16,
    crtc_y: i16,
    crtc_w: u16,
    crtc_h: u16,
}

fn atomic_add_con(
    gpu: &TegraDRM,
    req: &mut AtomicModeReq,
    con: &connector::Info,
    con_props: &HashMap<String, property::Info>,
    crtc: &crtc::Info,
    crtc_props: &HashMap<String, property::Info>,
    mode: &control::Mode,
    planes: &[(&PlaneConfig, &HashMap<String, property::Info>)],
) {
    req.add_property(
        con.handle(),
        con_props["CRTC_ID"].handle(),
        property::Value::CRTC(Some(crtc.handle())),
    );

    let blob = gpu.create_property_blob(mode).unwrap();
    req.add_property(crtc.handle(), crtc_props["MODE_ID"].handle(), blob);
    req.add_property(
        crtc.handle(),
        crtc_props["ACTIVE"].handle(),
        property::Value::Boolean(true),
    );

    for &(config, plane_props) in planes {
        req.add_property(
            config.plane,
            plane_props["FB_ID"].handle(),
            property::Value::Framebuffer(config.fb),
        );
        req.add_property(
            config.plane,
            plane_props["CRTC_ID"].handle(),
            property::Value::CRTC(Some(crtc.handle())),
        );
        req.add_property(
            config.plane,
            plane_props["SRC_X"].handle(),
            property::Value::UnsignedRange(config.src_x as _),
        );
        req.add_property(
            config.plane,
            plane_props["SRC_Y"].handle(),
            property::Value::UnsignedRange(config.src_y as _),
        );
        req.add_property(
            config.plane,
            plane_props["SRC_W"].handle(),
            property::Value::UnsignedRange((config.src_w as u64) << 16),
        );
        req.add_property(
            config.plane,
            plane_props["SRC_H"].handle(),
            property::Value::UnsignedRange((config.src_h as u64) << 16),
        );
        req.add_property(
            config.plane,
            plane_props["CRTC_X"].handle(),
            property::Value::SignedRange(config.crtc_x as _),
        );
        req.add_property(
            config.plane,
            plane_props["CRTC_Y"].handle(),
            property::Value::SignedRange(config.crtc_y as _),
        );
        req.add_property(
            config.plane,
            plane_props["CRTC_W"].handle(),
            property::Value::SignedRange(config.crtc_w as _),
        );
        req.add_property(
            config.plane,
            plane_props["CRTC_H"].handle(),
            property::Value::SignedRange(config.crtc_h as _),
        );
    }
}
