#!/usr/bin/bash
usage="$(basename "$0") [-h] [-d] 
helm install script

where:
    -h  show this help text
    -d  try run helm debugger"

if [ "$1" = "-h" ];then
    echo "$usage"
elif [ "$1" = "-d" ];then
    echo "run helm debug ..."
    helm install jj --debug --namespace=job-judge --create-namespace --dry-run ./helm
else
    echo "run helm ..."
    helm install jj --namespace=job-judge --create-namespace ./helm
fi

