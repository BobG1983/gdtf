pub(crate) mod rider;
pub(crate) mod screenshot;

#[cfg(test)]
mod test;

pub(in crate::dev::net_qa) use rider::drive_rider_captures;
pub(in crate::dev::net_qa) use screenshot::CaptureScreenshot;
