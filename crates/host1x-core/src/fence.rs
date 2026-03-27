pub trait Fence {
    type Output;
    type Error;

    fn wait(&self) -> Result<Self::Output, Self::Error>;
}
