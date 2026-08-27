pub(crate) mod screenshot;

#[cfg(test)]
mod test;

pub(in crate::dev::net_qa) use screenshot::CaptureScreenshot;
