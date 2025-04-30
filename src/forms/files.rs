use actix_multipart::form::{MultipartForm, tempfile::TempFile};

#[derive(MultipartForm)]
pub struct UploadFileForm {
    #[multipart(limit = "10MB")]
    pub image: TempFile,
}
