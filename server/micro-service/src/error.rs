pub enum Error {
    ConnectionFailed,
    Timeout,
    CheckFailed,
    ResourceLimit,
    Unknown,
}
pub type Result<T> = std::result::Result<T, Error>;
