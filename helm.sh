#!/usr/bin/bash
usage="$(basename "$0") [-h] [-d] 
helm install script

where:
    -h  show this help text
    -d  try run helm debugger
    -u  uninstall helm package"

if [ "$1" = "-h" ];then
    echo "$usage"
elif [ "$1" = "-d" ];then
    echo "run helm debug ..."
    helm install jj --debug --namespace=job-judge --create-namespace --dry-run ./helm
elif [ "$1" = "-u" ];then
    echo "uninstall helm ..."
    helm uninstall --namespace=job-judge jj
else
    echo "run helm ..."
    helm install jj --namespace=job-judge --create-namespace ./helm
fi

