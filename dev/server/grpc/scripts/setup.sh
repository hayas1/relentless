#!/bin/bash

sudo apt-get update -y && sudo apt-get upgrade -y
sudo apt-get install -y protobuf-compiler

go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest
