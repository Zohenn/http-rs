# http-rs
A very noob attempt at writing code in Rust.

### todo
- [x] HTTPS support
- [x] request listener, similar to the one present in native http module in Node.js
- [x] kindly do not return HTTP 400 on TCP FIN message
- [x] better HTTP method handling:
  - [x] 405 when requesting static content with method other than GET
  - [x] HEAD response without body
  - [x] OPTIONS requests
- [x] handling requests with non-empty body
- [ ] custom builder-pattern macro
- [x] keep-alive
- [ ] binary compilation target that serves static files only and reads config from config file
- [x] multithreading
- [x] allowing HTTPS and non-HTTPS traffic simultaneously

### mightdo
- [ ] HTTP/2 support
- [x] something similar to nginx rewrite rules or .htaccess files
