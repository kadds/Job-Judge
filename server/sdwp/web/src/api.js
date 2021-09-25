import axios from 'axios'
import store from './store/index'

let base_url = "http://localhost:6550/api"

let token = ''

const instance = axios.create({
    baseURL: base_url,
    timeout: 5000,
    headers: { 'Access-Control-Allow-Origin': '*' }
})

instance.interceptors.request.use(function (config) {
    return {
        ...config,
        headers: { ...config.headers, 'Token': token }
    }
}, function (error) {
    const res = error.request
    store.ui.errors.push(res.config.url, res.status, res.statusText, res.data)
    return Promise.reject(error)
})

instance.interceptors.response.use(function (response) {
    return response
}, function (error) {
    const res = error.response
    if (res && res.status === 401) {
        store.ui.login.show_dialog()
    } else {
        if (res !== undefined) {
            store.ui.errors.push(res.config.url, res.status, res.statusText, res.data)
        } else {
            store.ui.errors.push(error.config.url, -1, error.message, error.stack)
        }
    }
    return Promise.reject(error)
})

async function login(username, password) {
    let resp = await instance.post(base_url + '/user/login', { username, password })
    token = JSON.parse(resp).token
    return token
}

async function list_service() {
    let resp = await instance.get('/service/list')
    return resp.data.list
}

async function list_rpc(module, service = null, ins = null) {
    let url = `/service/rpcs?module=${encodeURI(module)}`
    if (service !== null) {
        url += `&service=${encodeURI(service)}`
    }
    if (ins !== null) {
        url += `&instance=${encodeURI(ins)}`
    }
    let resp = await instance.get(url)
    return resp.data
}

async function get_rpc(module, service, ins, method) {
    let resp = await instance.get(`/service/rpc?module=${encodeURI(module)}&service=${encodeURI(service)}&instance=${encodeURI(ins)}&method=${encodeURI(method)}`)
    return resp.data.rpc
}

async function invoke_rpc(module, service, ins, method, body) {
    let resp = await instance.post(`/service/invoke`, { module, service, instance: ins, method, body }, { timeout: 1000 * 60 })
    console.log(resp)
    return { data: resp.data, cost: parseInt(resp.headers.cost) }
}

export { list_service, list_rpc, get_rpc, invoke_rpc, login }
