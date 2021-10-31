#!/usr/bin/bash
usage="$(basename "$0") subcommand [options]
helm install script
subcommands:
  install:
    -d debug mode
    -f <file> install parameters
    -sha <hash> git sha tag
  update: 
    -f <file> install parameters
    -sha <hash> git sha tag
  uninstall: uninstall current package
  help: show this message"

declare -A flags
declare -A booleans

subcmd=$1
shift

while [ "$1" ];
do
    arg=$1
    if [ "${1:0:1}" == "-" ]
    then
      shift
      rev=$(echo "$arg" | rev)
      if [ -z "$1" ] || [ "${1:0:1}" == "-" ] || [ "${rev:0:1}" == ":" ]
      then
        bool=$(echo ${arg:1} | sed s/://g)
        booleans[$bool]=true
      else
        value=$1
        flags[${arg:1}]=$value
        shift
      fi
    else
      args+=("$arg")
      shift
    fi
done

if [ "$subcmd" = "install" ];then
    echo "run helm ..."
    cmd="helm install jj"
    if [[ -v booleans[d] ]]; then
        cmd="${cmd} --debug --dry-run"
    fi
    cmd="${cmd} --namespace=job-judge --create-namespace ./helm"
    if [[ -v flags["f"] ]]; then
        cmd="${cmd} -f=${flags["f"]}"
    fi
    if [[ -v flags["sha"] ]]; then
        cmd="${cmd} --set global.image.tag=${flags["sha"]}"
    fi
    echo $cmd
    eval $cmd
elif [ "$subcmd" = "uninstall" ];then
    echo "uninstall helm ..."
    cmd="helm uninstall --namespace=job-judge jj"
    echo $cmd
    eval $cmd
elif [ "$subcmd" = "update" ];then
    echo "update helm ..."
    cmd="helm upgrade --namespace=job-judge jj ./helm"
    if [[ -v flags["f"] ]]; then
        cmd="${cmd} -f=${flags["f"]}"
    fi
    if [[ -v flags["sha"] ]]; then
        cmd="${cmd} --set global.image.tag=${flags["sha"]}"
    fi
    echo $cmd
    eval $cmd
else
    echo "${usage}"
fi

