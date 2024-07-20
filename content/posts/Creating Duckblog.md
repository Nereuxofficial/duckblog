---
title: Creating Duckblog
date: 2023-02-09
draft: true
tags:
  - rust
  - webdev
  - axum
  - blog
readingTime: false
hideComments: false
url: /posts/creating-duckblog
description: Building a blog in Rust
---


Hey, it's been a while. I've been pretty busy with life and university, but I'm back with a new project.

I've changed the blog engine from [Hugo](https://gohugo.io/) to my own [duckblog](TODO) and in this post I'll explain 
some cool features and how I implemented them.

## Features



Calculating the reading time of a post is difficult, since it depends on the reader's reading speed and the length of the words in the post.
I decided to use the average reading speed of 200 words per minute and the average length of words in the English language of 5 characters.