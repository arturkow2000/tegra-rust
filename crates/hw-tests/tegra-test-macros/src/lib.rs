//! Macros for GR2D hardware tests
//!
//! [`test!`] internally calls into `rstest` but additionally uses supports
//! configures tokio, tracing spans, and supports tests generic over formats.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    FnArg, GenericParam, Generics, ItemFn, Pat, Type, TypeParamBound, WherePredicate,
    parse::Nothing, parse_macro_input,
};

const RGB_FORMATS: &[(&str, &str)] = &[("Argb8888", "argb8888"), ("Abgr8888", "abgr8888")];

#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
    parse_macro_input!(attr as Nothing);
    let func = parse_macro_input!(item as ItemFn);
    if !has_format_param(&func.sig.generics) {
        return plain(func);
    }

    // The wrappers call `__impl::<Format>`, so the format must be the only
    // generic parameter.
    if func.sig.generics.params.len() != 1 {
        return syn::Error::new_spanned(
            &func.sig.generics,
            "a format-generic test must have the format as its only generic parameter",
        )
        .to_compile_error()
        .into();
    }
    generic_over_formats(func)
}

/// Detects whether function has F: Format generic parameter.
/// Those functions will be parametrized on GR2D formats.
fn has_format_param(generics: &Generics) -> bool {
    let is_format = |bound: &TypeParamBound| {
        // currently we support RGB only
        matches!(bound, TypeParamBound::Trait(t)
            if t.path.segments.last().is_some_and(|s| s.ident == "RgbFormat"))
    };
    let predicates = generics.where_clause.iter().flat_map(|w| &w.predicates);

    generics.params.iter().any(|param| {
        let GenericParam::Type(tp) = param else {
            return false;
        };
        let bounds_param = |ty: &Type| {
            matches!(ty, Type::Path(p) if p.qself.is_none() && p.path.is_ident(&tp.ident))
        };
        tp.bounds.iter().any(is_format)
            || predicates.clone().any(|pred| {
                matches!(pred, WherePredicate::Type(pt)
                    if bounds_param(&pt.bounded_ty) && pt.bounds.iter().any(is_format))
            })
    })
}

fn configure_tokio() -> impl quote::ToTokens {
    quote! {
        #[::tokio_shared_rt::test(flavor = "multi_thread", shared = true)]
    }
}

/// A plain, non-generic test: rstest driver + a span named after the function +
/// the shared async runtime.
fn plain(func: ItemFn) -> TokenStream {
    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = func;
    let name = sig.ident.to_string();
    let tokio = configure_tokio();

    quote! {
        #[::rstest::rstest]
        #(#attrs)*
        #[::tracing::instrument(name = #name, skip_all)]
        #tokio
        #vis #sig #block
    }
    .into()
}

fn generic_over_formats(func: ItemFn) -> TokenStream {
    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = func;

    let name = sig.ident.to_string();
    let tokio = configure_tokio();

    // The arguments to forward from each wrapper to the impl, in order. They
    // are plain idents in every hardware test; anything else is unsupported.
    let mut arg_idents = Vec::new();
    for input in &sig.inputs {
        match input {
            FnArg::Typed(pt) => match &*pt.pat {
                Pat::Ident(pi) => arg_idents.push(pi.ident.clone()),
                other => {
                    return syn::Error::new_spanned(
                        other,
                        "format-generic test arguments must be plain identifiers",
                    )
                    .to_compile_error()
                    .into();
                }
            },
            FnArg::Receiver(r) => {
                return syn::Error::new_spanned(r, "a test cannot take `self`")
                    .to_compile_error()
                    .into();
            }
        }
    }

    // The hidden generic impl: the original signature and body, but renamed and
    // with the rstest `#[case]` param attributes stripped (it is a plain fn).
    let impl_name = format_ident!("__{}_impl", sig.ident);
    let mut impl_sig = sig.clone();
    impl_sig.ident = impl_name.clone();
    for input in &mut impl_sig.inputs {
        if let FnArg::Typed(pt) = input {
            pt.attrs.clear();
        }
    }

    // One concrete wrapper per format. It keeps the `#[case]` attributes (on the
    // function and its params) so rstest parametrizes it, drops the generics,
    // and forwards to the impl with the format pinned.
    let mut wrap_sig = sig.clone();
    wrap_sig.generics = Default::default();

    let wrappers = RGB_FORMATS.iter().map(|&(ty, suffix)| {
        let fmt_ty = format_ident!("{}", ty);
        let mut wrap_sig = wrap_sig.clone();
        wrap_sig.ident = format_ident!("{}_{}", sig.ident, suffix);
        quote! {
            #[::rstest::rstest]
            #(#attrs)*
            #[::tracing::instrument(name = #name, skip_all, fields(format = #ty))]
            #tokio
            #vis #wrap_sig {
                #impl_name::<::colorspace::#fmt_ty>(#(#arg_idents),*).await
            }
        }
    });

    quote! {
        #impl_sig #block
        #(#wrappers)*
    }
    .into()
}
