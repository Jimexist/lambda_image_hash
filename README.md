# Lambda Image Hash in Rust

[![CI](https://github.com/Jimexist/lambda_image_hash/actions/workflows/ci.yml/badge.svg)](https://github.com/Jimexist/lambda_image_hash/actions/workflows/ci.yml)

This is a simple AWS Lambda function that takes an image located in S3 and returns the hash of the image.

Your images have to be located in S3 already, and also accessible to the Lambda function itself.

## Step 1 - Create AWS Function

Go to [console][console] for a list of AWS Lambda functions.

Create one with _Amazon Linux 2023_ runtime in order to run Rust.

Make sure you have _Create a new role with basic Lambda permissions_ enabled, or you can choose to create the IAM role in advance.

The IAM role should have (by default) CloudWatch logs and Lambda basic execution permissions. But make sure to [add read access][iam] to the S3 bucket so that the function can access the images.

## Step 2 - Build and deploy

Firstly make sure you [install cargo lambda][bin].

To build a new version:

```bash
cargo lambda build --release
```

To upload an updated version, locate your Lambda's run role ARN [here][iam].

```bash
cargo lambda deploy \
--iam-role 'arn:aws:iam::xxxxxxxx:role/service-role/lambda_image_hash-role-mc59bxlm'
```

More details about AWS lambda in Rust can be found [here][guide].

## Step 3 - Invoke on the command line

On the project root, you can directly invoke the lambda function:

```bash
cargo lambda invoke --remote \
  --data-ascii '{"path": "file/path/to/s3"}' \
  --output-format json
```

Or if you have enabled HTTP endpoint access you can use `wget`, `curl`, or `Postman` to access.

The success response should look like this:

```json
{
  "hash_base64": "DCIoZGAwd2U",
  "image_size": [1200, 900],
  "time_elapsed": 0.19763084
}
```

## Algorithm used

Currently, the default algorithm used is [`Gradient`][algo], but you can supply a different one in request payload.

## Image formats

Currently only `webp` and `jpeg` file formats are used, in order to cut down binary size. It is controlled by the `image` crate's features gate.

[algo]: https://docs.rs/image_hasher/latest/image_hasher/enum.HashAlg.html
[console]: https://ap-southeast-1.console.aws.amazon.com/lambda/home
[bin]: https://www.cargo-lambda.info/guide/installation.html
[guide]: https://docs.aws.amazon.com/sdk-for-rust/latest/dg/lambda.html
[iam]: https://us-east-1.console.aws.amazon.com/iam/home#/roles
