import boto3, os, json

iam = boto3.client("iam")
sm = boto3.client("secretsmanager")

USER = os.environ["IAM_USER"]
ARN = os.environ["SECRET_ARN"]


def lambda_handler(event, ctx):
    new = iam.create_access_key(UserName=USER)["AccessKey"]
    sm.put_secret_value(
        SecretId=ARN,
        SecretString=json.dumps(
            {
                "aws_access_key_id": new["AccessKeyId"],
                "aws_secret_access_key": new["SecretAccessKey"],
            }
        ),
    )
    for k in iam.list_access_keys(UserName=USER)["AccessKeyMetadata"]:
        if k["AccessKeyId"] != new["AccessKeyId"]:
            iam.delete_access_key(UserName=USER, AccessKeyId=k["AccessKeyId"])
    return {"status": "rotated", "new_key": new["AccessKeyId"]}
