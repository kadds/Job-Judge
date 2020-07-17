const { Router } = require('express')
const { conn, m_vm } = require('../data')

let router = new Router()

router.get('/:name', async (req, rsp, next) => {
    try {
        var item = await m_vm.findByPk(req.params.name)
        if (item == null) {
            rsp.json({})
        }
        else {
            rsp.json(item.get())
        }
    }
    catch (e) {
        console.log(e)
        rsp.json({})
        return;
    }
})

router.put('/', async (req, rsp, next) => {
    try {
        await m_vm.create(req.body.vm)
    }
    catch (e) {
        console.log(e)
        return;
    }
})

router.post('/', (req, rsp, next) => {

})

router.get('/all', (req, rsp, next) => {

})

const vm = router
module.exports = vm