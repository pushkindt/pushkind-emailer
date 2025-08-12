use actix_multipart::form::{MultipartForm, tempfile::TempFile};

/// Form used to upload a single file via multipart request.
#[derive(MultipartForm)]
pub struct UploadFileForm {
    #[multipart(limit = "10MB")]
    pub image: TempFile,
}
