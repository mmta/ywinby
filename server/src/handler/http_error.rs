use actix_http::StatusCode;
use log::error;

pub trait IntoHttpError<T> {
    fn http_error(
        self,
        message: &str,
        status_code: StatusCode
    ) -> core::result::Result<T, actix_web::Error>;

    fn http_internal_error(self, message: &str) -> core::result::Result<T, actix_web::Error>
        where Self: std::marker::Sized
    {
        self.http_error(message, StatusCode::INTERNAL_SERVER_ERROR)
    }
    fn http_unauthorized_error(self, message: &str) -> core::result::Result<T, actix_web::Error>
        where Self: std::marker::Sized
    {
        self.http_error(message, StatusCode::UNAUTHORIZED)
    }
    fn http_not_found_error(self, message: &str) -> core::result::Result<T, actix_web::Error>
        where Self: std::marker::Sized
    {
        self.http_error(message, StatusCode::NOT_FOUND)
    }
}

impl<T, E: std::fmt::Debug> IntoHttpError<T> for core::result::Result<T, E> {
    fn http_error(
        self,
        message: &str,
        status_code: StatusCode
    ) -> core::result::Result<T, actix_web::Error> {
        match self {
            Ok(val) => Ok(val),
            Err(err) => {
                error!("http error returned: ({}) {}. Err debug: {:?}", status_code, message, err);
                Err(actix_web::error::InternalError::new(message.to_string(), status_code).into())
            }
        }
    }
}