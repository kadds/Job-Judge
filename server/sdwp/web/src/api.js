import axios from 'axios'

let base_url = "http://localhost:6550/api"

let token = ""

const instance = axios.create({
    baseURL: base_url,
    timeout: 2000,
    headers: { 'Token': token }
});

async function login(username, password) {
    let resp = await instance.post(base_url + '/user/login', { username, password })
    token = JSON.parse(resp).token
    return token
}

async function list_service() {
    let resp = await instance.get('/service/list')
    return resp.data.list
}

export { list_service, login }
