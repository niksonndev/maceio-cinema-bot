import {
  S3Client,
  CreateBucketCommand,
  HeadBucketCommand,
  type CreateBucketCommandInput,
  type BucketLocationConstraint,
} from '@aws-sdk/client-s3';

async function ensureBucket(): Promise<void> {
  const bucket = process.env.S3_BUCKET;
  if (!bucket) return;

  const client = new S3Client({
    region: process.env.AWS_REGION || 'us-east-1',
    endpoint: process.env.AWS_ENDPOINT_URL,
    forcePathStyle: true,
    credentials: {
      accessKeyId: process.env.AWS_ACCESS_KEY_ID || 'test',
      secretAccessKey: process.env.AWS_SECRET_ACCESS_KEY || 'test',
    },
  });

  try {
    await client.send(new HeadBucketCommand({ Bucket: bucket }));
  } catch {
    const region = process.env.AWS_REGION || 'us-east-1';
    const input: CreateBucketCommandInput = { Bucket: bucket };
    if (region !== 'us-east-1') {
      input.CreateBucketConfiguration = {
        LocationConstraint: region as BucketLocationConstraint,
      };
    }
    await client.send(new CreateBucketCommand(input));
  }
}

await ensureBucket();
