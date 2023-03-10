FROM ubuntu:20.04

RUN apt-get update && apt-get install -y python3 python3-pip
RUN apt-get install -y curl

ENV PRIVATE_KEY_1=0xf5c746b15e3bdd6bf8c516031a15de16ea161ce10552635abb27c9bd16e0a04a
COPY ./tests/requirements.txt /
WORKDIR /
RUN pip3 install -r requirements.txt

COPY . /opt
WORKDIR /opt

ENTRYPOINT [ "/opt/entrypoint.sh" ]
