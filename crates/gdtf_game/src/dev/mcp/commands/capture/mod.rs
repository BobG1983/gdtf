pub(crate) mod screenshot;

#[cfg(test)]
mod test;

pub(in crate::dev::mcp) use screenshot::CaptureScreenshot;
