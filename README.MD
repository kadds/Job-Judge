# Job Judge
An online code judger.  
Built from rust.  
Deployed by k8s/k3s.  
    
[![Code-Linter](https://github.com/kadds/Job-Judge/actions/workflows/lint.yaml/badge.svg)](https://github.com/kadds/Job-Judge/actions/workflows/lint.yaml) 
[![Build](https://github.com/kadds/Job-Judge/actions/workflows/build.yaml/badge.svg)](https://github.com/kadds/Job-Judge/actions/workflows/build.yaml)

## List of services
* gateway  
    gets http request and sends results
* usersvr  
    table 'user' database
* sessionsvr  
    holding tokens for users
* idsvr  
    id generators for distributed environments.  
    Stateful service
* compilationsvr  
* containersvr  
* judgesvr  
* runsvr  
* sdwp  
    `server debug web page` can send rpc to services
* helm
    helm package directory