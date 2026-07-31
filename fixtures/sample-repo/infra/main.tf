resource "aws_s3_bucket" "logs" {
  bucket = "demo-logs"
}
data "aws_ami" "ubuntu" {
  most_recent = true
}
