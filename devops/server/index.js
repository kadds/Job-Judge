const express = require('express')
const mode = require('./routers/mode.js')
const pipeline = require('./routers/pipeline.js')
const vm = require('./routers/vm.js')
const { init } = require('./data')
const bodyParser = require('body-parser')


function start() {
    const app = express()
    app.on('error', (val) => {
        console.log(val)
    })

    app.use(bodyParser.json())
    app.use('/', (req, rsp, next) => {
        rsp.set("Content-Type", "application/json")
        next()
    })

    app.use('/mode', mode)
    app.use('/pipeline', pipeline)
    app.use('/vm', vm)

    app.listen(8077)
    console.log("start listen")
}

init().then(() => {
    start()
})
