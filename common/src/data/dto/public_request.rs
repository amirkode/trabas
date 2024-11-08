use core::fmt;

use serde::{Deserialize, Serialize};

// still don't know whether these enums are required yet
pub enum ReqMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE
}

pub enum ReqBodyType {
    RAW,
    FORM // TODO: support for multipart request
}

// after few considerations, it doesn't really to breakdown explicit request specs
// serialization of the actual request would do the job 
#[derive(Serialize, Deserialize, Clone)]
pub struct PublicRequest {
    pub id: String,
    pub data: Vec<u8>
}

impl fmt::Display for PublicRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write the desired string representation of the struct
        write!(f, "[id: {})", self.id)
    }
}
