#!/bin/sh
rm -r blog-serverless/public
cp -r public/ blog-serverless/public
cd blog-serverless
cargo shuttle deploy