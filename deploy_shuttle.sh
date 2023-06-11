#!/bin/sh
cargo r -r -- --ssg
rm -r blog-serverless/public
cp -r public/ blog-serverless/public
cd blog-serverless
cargo shuttle deploy --allow-dirty
