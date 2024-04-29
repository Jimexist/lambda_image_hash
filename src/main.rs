use aws_config::BehaviorVersion;
use aws_sdk_s3::operation::get_object::GetObjectOutput;
use image::io::Reader as ImageReader;
use image::GenericImageView;
use image_hasher::{HashAlg, HasherConfig};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TypedError {
    #[error("Failed to retrieve from S3")]
    S3Get,
    #[error("Failed to download from S3: `{0}`")]
    S3Download(String),
    #[error("Invalid image format `{0}` guessed")]
    InvalidFormat(String),
}

#[derive(Deserialize)]
struct Request {
    path: String,
    algo: Option<HashAlg>,
}

#[derive(Debug, Serialize)]
struct Response {
    hash_base64: String,
    algo: HashAlg,
    image_size: (u32, u32),
    time_elapsed: f64,
}

async fn download_from_s3(
    s3_client: &aws_sdk_s3::Client,
    bucket_name: &str,
    key: &str,
) -> Result<GetObjectOutput, TypedError> {
    let response = s3_client
        .get_object()
        .bucket(bucket_name)
        .key(key)
        .send()
        .await;
    match response {
        Ok(output) => {
            tracing::info!(
                key = %key,
                "data successfully retrieved from S3",
            );
            Ok(output)
        }
        Err(err) => {
            tracing::error!(
                err = %err,
                key = %key,
                "failed to retrieve data from S3"
            );
            return Err(TypedError::S3Get);
        }
    }
}

#[tracing::instrument(skip(s3_client, event), fields(req_id = %event.context.request_id))]
async fn put_object(
    s3_client: &aws_sdk_s3::Client,
    bucket_name: &str,
    event: LambdaEvent<Request>,
) -> Result<Response, TypedError> {
    tracing::info!("handling a request");

    let key = event.payload.path.clone();
    let response = download_from_s3(&s3_client, &bucket_name, &key).await?;

    let data = response
        .body
        .collect()
        .await
        .map_err(|e| TypedError::S3Download(e.to_string()))?;

    let bytes = data.into_bytes();

    // Load image from bytes
    let img = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| TypedError::InvalidFormat(e.to_string()))?
        .decode()
        .map_err(|e| TypedError::InvalidFormat(e.to_string()))?;

    // get image size
    let (width, height) = img.dimensions();

    // get hashing timing
    let algo = event.payload.algo.unwrap_or(HashAlg::Gradient);
    let hasher = HasherConfig::new().hash_alg(algo).to_hasher();
    let start = std::time::Instant::now();
    let hash = hasher.hash_image(&img);
    let elapsed = start.elapsed();

    Ok(Response {
        hash_base64: hash.to_base64(),
        image_size: (width, height),
        algo,
        time_elapsed: elapsed.as_secs_f64(),
    })
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        // disable printing the name of the module in every log line.
        .with_target(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();

    let bucket_name = std::env::var("BUCKET_NAME")
        .expect("A BUCKET_NAME must be set in this app's Lambda environment variables.");

    // Initialize the client here to be able to reuse it across
    // different invocations.
    //
    // No extra configuration is needed as long as your Lambda has
    // the necessary permissions attached to its role.
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let s3_client = aws_sdk_s3::Client::new(&config);

    lambda_runtime::run(service_fn(|event: LambdaEvent<Request>| async {
        put_object(&s3_client, &bucket_name, event).await
    }))
    .await
}
