# Open-source Rules Engine Editor with build in Simulator

URL: https://editor.gorules.io

## Running via Docker

Running locally:
```bash
docker run -p 3000:3000 --platform=linux/amd64 gorules/editor
```

Repository:
https://hub.docker.com/r/gorules/editor

## Quickstart

Run rust backend:
```bash
make watch
```

Run frontend:
```bash
npm i pnpm -g
pnpm i
pnpm dev
```

test


### Local HTTPS

To create a local HTTPS certificate:
```bash
brew install mkcert

mkcert --install
cd cert && mkcert localhost
```


```
docker build -t rule-engine:v0.0.1-beta .
```
```
docker tag rule-engine:v0.0.1-beta 5.182.17.45:5002/rule-engine:v0.0.1-beta
```
```
docker push 5.182.17.45:5002/rule-engine:v0.0.1-beta
```

docker run -p 3000:3000 --platform=linux/amd64 gorules/editor


docker  run -t rule-engine:v0.0.1-beta -p 5173:3000 --platform=linux/amd64 --name rule-engine


docker run -d --name rule-engine -p 5173:3000 --platform linux/amd64  rule-engine:v0.0.1-beta

