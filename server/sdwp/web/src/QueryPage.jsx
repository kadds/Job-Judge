import { Text, Checkbox, Toggle, CheckboxVisibility, Dropdown, Spinner, DetailsList, DetailsListLayoutMode, SelectionMode, TextField, Separator, PrimaryButton, IconButton, ContextualMenuItemType, Callout, Dialog, DialogFooter, DefaultButton, DialogType } from '@fluentui/react'
import React, { Fragment, useCallback, useEffect, useRef, useState } from 'react'
import { list_rpc, get_rpc, invoke_rpc } from './api'
import ui from './store/ui'
import JsonView from './JsonView'
import { motion, AnimatePresence } from "framer-motion"
import { inject, observer } from 'mobx-react'
import axios from 'axios'

const message_variants = {
    initial: {
        y: -30,
        opacity: 0,
        scaleY: 0.5,
        transformOrigin: 'center top',
    },
    animate: {
        y: 0,
        opacity: 1,
        scaleY: 1,
    },
    exit: {
        y: 0,
        opacity: 0,
        scaleY: 0.2
    },
}

const Enum = ({ info, message, data, dataUpdate, path }) => {
    let m = info.relate_schema[message]
    let options = m.enums.map(item => {
        return { key: item.name, text: `${item.name} (${item.pos})` }
    })
    return (
        <div className='enum'>
            <Dropdown onFocus={() => { ui.hint.set(path) }} onBlur={() => ui.hint.set('')}
                options={options} selectedKey={data} onChange={(e, value) => dataUpdate(value.key)}></Dropdown>
        </div>
    )
}

const NumTextBox = ({ type, data, dataUpdate, path }) => {
    const getValue = value => {
        if (value === null || value === '') {
            return null
        }
        if (type === 'i32' || type === 'i64') {
            let re = /^[+-]?\d+$/
            if (!value.match(re)) {
                return NaN
            }
            return parseInt(value)
        } else if (type === 'u32' || type === 'u64') {
            let re = /^\+?\d+$/
            if (!value.match(re)) {
                return NaN
            }
            return parseInt(value)
        } else {
            let re = /^[+-]?\d*\.?\d*$/
            if (!value.match(re)) {
                return NaN
            }
            return parseFloat(value)
        }
    }
    const onGetErrorMessage = (value) => {
        const val = getValue(value)
        if (isNaN(val)) {
            return 'Not a number'
        }
        dataUpdate(val)
        return ''
    }
    const onChange = (e, value) => {
        setInput(value)
    }

    let value = data
    if (value === null) {
        value = ''
    }
    const [input, setInput] = useState(value)

    useEffect(() => {
        if (data !== input) {
            setInput(data === null ? '' : data)
        }
    }, [data])

    return (
        <div className='num-input'>
            <TextField onFocus={() => { ui.hint.set(path) }} onBlur={() => ui.hint.set(null)}
                value={input} onChange={onChange} validateOnLoad={false}
                type="number"
                validateOnFocusOut={true}
                onGetErrorMessage={onGetErrorMessage} />
        </div>
    )
}

const stringMenu = [
    {
        key: 'Multi',
        text: 'Multi-line',
        iconProps: { iconName: 'MoreVertical' }
    },
    {
        key: 'Single',
        text: 'Single-line',
        iconProps: { iconName: 'More' }
    },
    {
        key: 'd0',
        itemType: ContextualMenuItemType.Divider
    },
    {
        key: 'Copy',
        text: 'Copy',
        iconProps: { iconName: 'Copy' }
    },
    {
        key: 'Parse',
        text: 'Parse',
        iconProps: { iconName: 'FileTemplate' }
    },
    {
        key: 'd1',
        itemType: ContextualMenuItemType.Divider
    },
    {
        key: 'From',
        text: 'Load',
        iconProps: { iconName: 'OpenFile' }
    },
]


const StringTextBox = ({ data, dataUpdate, path, isBytes }) => {
    const [multi, setMulti] = useState(false)
    const ref = useRef()
    const onChange = (e, value) => {
        dataUpdate(value)
    }
    const onPreviewClick = () => {
        ui.callout.set(data, ref.current)
    }
    const onMenuClick = (a, { key }) => {
        if (key === 'Multi') {
            setMulti(true)
        } else if (key === 'Single') {
            setMulti(false)
        } else if (key === 'Copy') {
            navigator.clipboard.writeText(data)
        } else if (key === 'Parse') {
            (async () => {
                const text = await navigator.clipboard.readText()
                if (text.indexOf('\n') >= 0) {
                    setMulti(true)
                }
                dataUpdate(text)
            })()
        } else if (key === 'From') {
            ui.dialog.open(`Select data from '${path}'`, result => {
                dataUpdate(result)
            })
        } else {
            console.error(key)
        }
    }
    let value = data
    if (value === null) {
        value = ''
    }
    return (
        <div style={{ display: 'flex' }}>
            <TextField onFocus={() => { ui.hint.set(path) }} onBlur={() => ui.hint.set(null)}
                value={value} rows={3} onChange={onChange} multiline={multi} />
            <div className={'string-textbox-buttons ' + (multi ? 'multi' : 'single')}>
                <IconButton menuProps={{ items: stringMenu.filter(items => items.key !== (multi ? 'Multi' : 'Single')), onItemClick: onMenuClick }} />
                <span ref={ref}>
                    <IconButton iconProps={{ iconName: 'Preview' }} onClick={onPreviewClick} />
                </span>
            </div>
        </div>
    )
}

const TypeInputBox = ({ type, info, data, dataUpdate, path }) => {
    if (type.type === 'String') {
        return (
            <StringTextBox data={data} dataUpdate={dataUpdate} path={path} />
        )
    } else if (type.type === 'Bytes') {
        return (
            <StringTextBox isBytes={true} data={data} dataUpdate={dataUpdate} path={path} />
        )
    } else if (type.type === 'Bool') {
        return (
            <Toggle onFocus={() => { ui.hint.set(path) }} onBlur={() => ui.hint.set(null)}
                value={data} onChange={(e, val) => dataUpdate(val)} />
        )
    } else if (type.type === 'Int32' || type.type === 'Sfixed32') {
        return (
            <NumTextBox data={data} dataUpdate={dataUpdate} type='i32' path={path} />
        )
    } else if (type.type === 'Int64' || type.type === 'Sfixed64') {
        return (
            <NumTextBox data={data} dataUpdate={dataUpdate} type='i64' path={path} />
        )
    } else if (type.type === 'Uint32' || type.type === 'Fixed32') {
        return (
            <NumTextBox data={data} dataUpdate={dataUpdate} type='u32' path={path} />
        )
    } else if (type.type === 'Uint64' || type.type === 'Fixed64') {
        return (
            <NumTextBox data={data} dataUpdate={dataUpdate} type='u64' path={path} />
        )
    } else if (type.type === 'Float') {
        return (
            <NumTextBox data={data} dataUpdate={dataUpdate} type='f32' path={path} />
        )
    } else if (type.type === 'Double') {
        return (
            <NumTextBox data={data} dataUpdate={dataUpdate} type='f64' path={path} />
        )
    } else if (type.type === 'Message') {
        return (
            <MessageRender
                message={type.relate}
                info={info}
                data={data}
                dataUpdate={dataUpdate}
                path={path} />
        )
    } else if (type.type === 'Enum') {
        return (
            <Enum message={type.relate} info={info} data={data} dataUpdate={dataUpdate} path={path} />
        )
    } else {
        return (
            <Text>Unknown type</Text>
        )
    }
}

const FieldRender = ({ item, info, data, dataUpdate, path }) => {
    const checked = data !== undefined
    const onCheckedChange = () => {
        if (!checked) {
            dataUpdate(null)
        } else {
            dataUpdate(undefined)
        }
    }
    let typename = item.ktype.type
    if (typename === 'Message' || typename === 'Enum') {
        typename += ' ' + item.ktype.relate
    }
    const name = `${item.json_name} (${typename})`
    if (item.label === 'Optional' || item.label === 'Required') {
        return (
            <motion.div {...message_variants} className='field'>
                <Checkbox className='field-label' label={name} checked={checked} onChange={onCheckedChange} />
                <AnimatePresence>
                {
                    checked && (
                            <motion.div {...message_variants} className='field-box'>
                                <TypeInputBox type={item.ktype} info={info} data={data}
                                    dataUpdate={dataUpdate} path={path} />
                            </motion.div>
                    )
                }
                </AnimatePresence>
            </motion.div>
        )
    } else if (item.label === 'Repeated') {
        let items = data
        if (items === null) {
            items = []
        }
        const onRepeatedAdd = () => {
            if (items === undefined) {
                items = []
            }
            items.push(null)
            dataUpdate(items)
        }
        const onRepeatedRemove = (index) => {
            items.splice(index, 1)
            dataUpdate(items)
        }
        const repeatedUpdate = (data, index) => {
            items[index] = data
            dataUpdate(items)
        }
        return (
            <motion.div {...message_variants} className='field'>
                <div className='field-line'>
                    <Checkbox className='field-label' label={name} checked={checked} onChange={onCheckedChange} />
                    <IconButton iconProps={{ iconName: 'Add' }} onClick={() => onRepeatedAdd()} className='field-icon' />
                </div>
                <AnimatePresence>
                {
                    checked && items.map((it, index) => (
                        <motion.div {...message_variants} className='field-group' key={index}>
                            <div className='field-group-icon'>
                                <IconButton iconProps={{ iconName: 'Clear' }} onClick={() => onRepeatedRemove(index)} className='field-icon' />
                            </div>
                            <div className='field-group-inner field-box'>
                                <TypeInputBox type={item.ktype} info={info} data={it} dataUpdate={data => repeatedUpdate(data, index)}
                                    path={path + '/' + index} />
                            </div>
                        </motion.div>
                    ))
                }
                </AnimatePresence>
            </motion.div>
        )
    } else {
        return null
    }
}

const MessageRender = ({ info, message, data, dataUpdate, path }) => {
    let m = info.relate_schema[message]
    const update = (d, item) => {
        if (data === null) {
            if (d !== undefined) {
                data = {}
            }
        }
        if (d === undefined) {
            Reflect.deleteProperty(data, item.json_name)
        } else {
            data[item.json_name] = d
        }
        dataUpdate(data)
    }

    return (
        <div className='message'>
            <AnimatePresence>
            {
                m && m.fields.map(item => (
                    <FieldRender key={item.json_name}
                        data={data !== undefined && data !== null ? data[item.json_name] : undefined}
                        dataUpdate={d => update(d, item)} item={item} info={info}
                        path={path + '/' + item.json_name}></FieldRender>
                ))
            }
            </AnimatePresence>
        </div>
    )
}

const QueryPage = inject('store')(observer(({ store, api, tab }) => {
    const [rpcList, setRpcList] = useState([])
    const [serviceList, setServiceList] = useState([])
    const [serviceSelection, setServiceSelection] = useState(null)
    const [instanceList, setInstanceList] = useState([])
    const [instanceSelection, setInstanceSelection] = useState(null)
    const [rpcInfo, setRpcInfo] = useState(null)
    const [resultData, setResultData] = useState(null)
    const [method, setMethod] = useState(null)
    const [updateFlag, setUpdateFlag] = useState(0)
    const [requestData, setRequestData] = useState({})
    const [editJson, setEditJsonInner] = useState(false)
    const [cost, setCost] = useState(null)
    const [dialogData, setDialogData] = useState({ uri: '', prefix: '', base64: null })

    const setEditJson = val => {
        setRequestData(JSON.parse(JSON.stringify(requestData)))
        setEditJsonInner(val)
    }

    useEffect(() => {
        (async () => {
            ui.tab.loading_tab(tab)
            const data = await list_rpc(api, serviceSelection, instanceSelection)
            setRpcList(data.rpcs)
            setServiceList(data.services.map(item => { return { key: item, text: item } }))
            setServiceSelection(data.service)
            setInstanceList(data.instances.map(item => { return { key: item, text: item } }))
            setInstanceSelection(data.instance)
            ui.tab.finish_loading(tab)
        })()
    }, [updateFlag])

    const onInvoked = useCallback(async m => {
        if (serviceSelection === null || instanceSelection === null || m === null) {
            return
        }
        const data = await get_rpc(api, serviceSelection, instanceSelection, m)
        setRpcInfo(data)
        setMethod(m)
        setRequestData({})

    }, [serviceSelection, instanceSelection])

    const onInstanceChange = (e, item) => {
        setInstanceSelection(item.key)
        setUpdateFlag(updateFlag + 1)
    }
    const onServiceChange = (e, item) => {
        setServiceSelection(item.key)
        setUpdateFlag(updateFlag + 1)
        setRequestData({})
        setMethod(null)
        setRpcInfo(null)
    }
    const onInvokeClick = () => {
        (async () => {
            ui.tab.loading_tab(tab)
            const start = Date.now()
            const { data, cost } = await invoke_rpc(api, serviceSelection, instanceSelection, method, requestData)
            setResultData(data)
            ui.tab.finish_loading(tab)
            const end = Date.now()
            const delta = end - start
            setCost({ client: delta, server: cost })
        })()
    }
    const onInputDialog = () => {
        let input = document.createElement('input')
        input.type = 'file'
        input.onchange = e => {
            const file = e.target.files[0]
            let reader = new FileReader()
            reader.readAsDataURL(file)

            reader.onload = () => {
                const content = reader.result
                let pos = content.indexOf(',')
                let base64 = content
                if (pos >= 0) {
                    pos += 1
                    base64 = content.substr(pos, content.length - pos)
                    const prefix = content.substr(0, pos)
                    setDialogData({ ...dialogData, prefix, base64, loading: false })
                } else {
                    setDialogData({ ...dialogData, prefix: '', base64, loading: false })
                }
            }
            reader.onerror = (e) => {
                ui.errors.push(file.name, -1, 'error load local file', e + '')
            }
            setDialogData({ uri: file.name, prefix: null, base64: null, loading: true })
        }
        input.click()
    }
    const onLoadURI = async () => {
        setDialogData({ ...dialogData, loading: true, prefix: null, })
        try {
            const data = await axios.get(dialogData.uri, { responseType: 'blob', headers: { 'Access-Control-Allow-Origin': '*' } })
            let reader = new window.FileReader()
            reader.readAsDataURL(data.data)
            reader.onload = () => {
                const content = reader.result
                let pos = content.indexOf(',')
                let base64 = content
                if (pos >= 0) {
                    pos += 1
                    base64 = content.substr(pos, content.length - pos)
                    const prefix = content.substr(0, pos)
                    setDialogData({ ...dialogData, prefix, base64, loading: false })
                } else {
                    setDialogData({ ...dialogData, prefix: '', base64, loading: false })
                }
            }
        } catch (e) {
            console.log(e.config)
            ui.errors.push(e.config.url, -1, 'error load file', e + '')
        }
    }
    return (
        <div className='query-content'>
            <div className='query-list-wrapper'>
                <Dropdown placeholder="Select a service"
                    label="Service"
                    options={serviceList}
                    selectedKey={serviceSelection}
                    onChange={onServiceChange}
                />
                <Dropdown placeholder="Select a instance"
                    label="Instance"
                    options={instanceList}
                    selectedKey={instanceSelection}
                    onChange={onInstanceChange} />
                <DetailsList
                    className='detail-list'
                    items={rpcList}
                    checkboxVisibility={CheckboxVisibility.hidden}
                    onItemInvoked={onInvoked}
                    selectionPreservedOnEmptyClick={true}
                    columns={[{
                        key: 'method',
                        name: 'Methods',
                        isIconOnly: false,
                        fieldName: 0,
                        isSizeable: false,
                        onRender: item => (<Text variant='medium'>{item}</Text>)
                    }]}
                    layoutMode={DetailsListLayoutMode.justified}
                    isHeaderVisible={true}
                    selectionMode={SelectionMode.single}
                />
            </div>
            <Separator vertical />
            <div className='query-main'>
                <div className='query-main-header'>
                    <Text variant='large'>.{serviceSelection}.{method}</Text>
                    <Toggle checked={editJson} onChange={() => setEditJson(!editJson)} inlineLabel label='View json' /></div>
                <div className='input-content-container'>
                    {
                        rpcInfo && (
                            <Fragment>
                                <div style={{ overflow: 'auto' }}>
                                    <div className={!editJson ? 'show' : 'hide'}>
                                        <MessageRender path='' data={requestData} dataUpdate={data => setRequestData(JSON.parse(JSON.stringify(data)))} message={rpcInfo.request_typename} info={rpcInfo} />
                                    </div>
                                    <div className={editJson ? 'show' : 'hide'}>
                                        <JsonView message={rpcInfo.request_typename} info={rpcInfo} object={requestData}
                                            objectUpdate={data => setRequestData(data)} editable />
                                    </div>
                                </div>
                                <PrimaryButton onClick={() => onInvokeClick()} className='send-button' text='Send' />
                            </Fragment>
                        )
                    }
                </div>
            </div>
            <Separator vertical />
            <div className='query-result'>
                {cost && (
                    <div className='query-time'>
                        <Text>Client side cost: {cost.client}ms</Text>
                        <Text>Server side cost: {cost.server}ms</Text>
                    </div>
                )}
                <JsonView object={resultData} />
            </div>
            {store.ui.callout.data !== null && (
                <Callout calloutMaxWidth='40%' target={store.ui.callout.target} onDismiss={() => ui.callout.clear()}>
                    <div style={{ margin: 20 }}>
                        <Text variant='xLargePlus' nowrap block> Preview </Text>
                        <div style={{ marginTop: 20 }}>
                            <img width='100%' src={'data:image/png;base64,' + store.ui.callout.data}></img>
                        </div>
                    </div>
                </Callout>
            )}
            <Dialog
                dialogContentProps={{ title: store.ui.dialog.hint, type: DialogType.normal }}
                hidden={!store.ui.dialog.show}
                onDismiss={() => ui.dialog.cancel()}
                minWidth='40%'
            >
                <div className='inner-dialog'>
                    <TextField inline label='URI' value={dialogData.uri} onChange={(e, data) => setDialogData({ ...dialogData, uri: data })} />
                    <div className='dialog-buttons'>
                        <DefaultButton text='Select local file' onClick={onInputDialog} />
                        <DefaultButton text='Load uri' onClick={onLoadURI} />
                    </div>
                    <div className='dialog-preview'>
                        {
                            dialogData.loading && (<Spinner />)
                        }
                        {
                            dialogData.prefix && (
                                <img className='img-view' width='90%' src={dialogData.prefix + dialogData.base64} />
                            )
                        }
                    </div>
                </div>
                <DialogFooter>
                    <PrimaryButton onClick={() => ui.dialog.commit(dialogData.base64)} text="OK" />
                    <DefaultButton onClick={() => ui.dialog.cancel()} text="Cancel" />
                </DialogFooter>
            </Dialog>
        </div >)
}))

export default QueryPage