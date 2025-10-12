from atmoz/sftp:alpine

RUN addgroup -g 1001 sftptest && \
	adduser -D -u 1001 -G sftptest -s /bin/sh sftptest

RUN echo "sftptest:pass" | chpasswd

RUN mkdir -p /home/sftptest && \
    chown root:root /home/sftptest && \
    chmod 755 /home/sftptest

RUN mkdir -p /home/sftptest/upload && \
    chown sftptest:sftptest /home/sftptest/upload && \
    chmod 755 /home/sftptest/upload

RUN echo "Sample file content" > /home/sftptest/upload/sample.txt && \
    echo "Test file content" > /home/sftptest/test.txt && \
    chown sftptest:sftptest /home/sftptest/upload/*
