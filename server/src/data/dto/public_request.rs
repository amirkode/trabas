use std::collections::HashMap;

enum ReqMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE
}

enum ReqBodyType {
    RAW,
    FORM // TODO: support for multipart request
}

pub struct PublicRequest {
    path: String,
    parameters: HashMap<String, String>,
    headers: HashMap<String, String>,
    body: String,
    body_type: ReqBodyType,
    method: ReqMethod
}