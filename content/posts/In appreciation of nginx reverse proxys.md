---
title: "In appreciation of nginx reverse proxys"
date: "2023-11-03"
tags: ["nginx", "ops", "sysadmin", "webserver"]
description: "How to painlessly use HTTPS with your web services"
draft: false
url: /posts/in-appreciation-nginx-reverse-proxy
---


When i first wanted to deploy my web services i was overwhelmed, simply because there are so many ways to do it. Some more secure and everyone has their own way... So here is my simple setup i use for pretty much every project i host on my own [Hetzner](https://www.hetzner.com/) Server:
## Prerequisites:
- nginx setup like [here](https://www.digitalocean.com/community/tutorials/how-to-configure-nginx-as-a-reverse-proxy-on-ubuntu-22-04)
- certbot installed and set up
## Steps:
1. Set up a simple web server(preferably with [docker-compose](https://docs.docker.com/compose/), don't bother yourself with certificates, setting up ports, accepting the right URL etc. Just bind to 127.0.0.1:YOUR_PORT to some port that's free.
2. Create nginx configs via(This assumes you've already set your DNS Options properly):
```bash
certbot certonly --nginx
```
3. Set up a really simple https-only nginx configuration while using your global defaults for proxying, blocklists etc.
```nginx
server {
   listen 80;
   server_name YOUR_DOMAIN;
   return 301 https://$server_name$request_uri;
}
server{
    listen 443 ssl;
    server_name YOUR_DOMAIN;
    
    ssl_certificate /etc/letsencrypt/live/YOUR_DOMAIN/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/YOUR_DOMAIN/privkey.pem;

    access_log /var/log/nginx/YOUR_DOMAIN.access.log;
    error_log /var/log/nginx/YOUR_DOMAIN.error.log;
    location / {
        proxy_pass http://127.0.0.1:YOUR_PORT;
    }
}   
```
and replace everything in `YOUR_DOMAIN` and `YOUR_PORT` with your configuration options.
4. Restart nginx via `systemctl restart nginx`
   (Of course you should then monitor the error log and test whether it is working alright. There may be some security precautions(like Client Body size) preventing your service from working properly)

The nice thing about this is that it is a simple webserver Setup that abstracts away the differences in setting up certificates in different webservers, adds an additional layer of security, let's you benefit from the [tooling and documentation around nginx](https://github.com/agile6v/awesome-nginx) and is really painless.

It should be noted that nginx is written in C and has had multiple security issues in the past, however as it is widely used it is really well tested. Maybe that's a fun project to rewrite in Rust.

![visualization of the reverse proxy](images/nginx-reverse-proxy.avif)

If this helped you, consider supporting me [here](https://github.com/sponsors/Nereuxofficial) and if you have any feedback you can reach me on [Mastodon](https://infosec.exchange/@Nereuxofficial).