# Job Judge
An online homework/code judge system.  
Build by rust.  
[![Code-Linter](https://github.com/kadds/Job-Judge/actions/workflows/lint.yaml/badge.svg)](https://github.com/kadds/Job-Judge/actions/workflows/lint.yaml) 
[![Build](https://github.com/kadds/Job-Judge/actions/workflows/build.yaml/badge.svg)](https://github.com/kadds/Job-Judge/actions/workflows/build.yaml)

## List of services
* gateway  
    gets http request and sends results
* usersvr  
* sessionsvr  
    holding tokens for users
* idsvr  
    id generators for distributed environments
* compilationsvr  
* containersvr  
* judgesvr  
* runsvr  
* sdwp  
    `server debug web page` can send rpc to services