#!/bin/zsh
#
#docker build --platform linux/arm64 -t test-sftp-server . 

docker run -d \
	--name sftp-test \
	-p 2222:22 \
	test-sftp-server sftptest:pass:1001:1001
