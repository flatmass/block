use actix_web::{dev::Payload, error, multipart, Error};
use futures::{future, Future, Stream};

type BoxStream<T> = Box<dyn Stream<Item = T, Error = Error>>;
type BoxFuture<T> = Box<dyn Future<Item = T, Error = Error>>;
type MultipartItem = multipart::MultipartItem<Payload>;
type MultipartField = multipart::Field<Payload>;

pub fn handle_multipart_item(item: MultipartItem) -> BoxStream<(String, Vec<u8>)> {
    match item {
        MultipartItem::Field(field) => {
            let name = field
                .content_disposition()
                .expect("No content disposition")
                .get_name()
                .expect("No name")
                .to_owned();
            Box::new(load_in_memory(name, field).into_stream())
        }
        MultipartItem::Nested(mp) => Box::new(
            mp.map_err(error::ErrorInternalServerError)
                .map(handle_multipart_item)
                .flatten(),
        ),
    }
}

fn load_in_memory(name: String, field: MultipartField) -> BoxFuture<(String, Vec<u8>)> {
    Box::new(
        field
            .fold((name, Vec::new()), |mut data, bytes| {
                data.1.extend_from_slice(bytes.as_ref());
                future::ok(data)
                    .map_err(|e| error::MultipartError::Payload(error::PayloadError::Io(e)))
            })
            .from_err(),
    )
}
