import { Text, Checkbox, Toggle, CheckboxVisibility, Dropdown, DetailsList, DetailsListLayoutMode, SelectionMode, TextField, Separator, PrimaryButton, IconButton } from '@fluentui/react'
import React, { useCallback, useEffect, useState } from 'react'
import { list_rpc, get_rpc, invoke_rpc } from './api'
import ui from './store/ui'
import JsonView from './JsonView'

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
            return 'Not an number'
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
                validateOnFocusOut={true}
                onGetErrorMessage={onGetErrorMessage} />
        </div>
    )
}

const StringTextBox = ({ data, dataUpdate, path }) => {
    const onChange = (e, value) => {
        dataUpdate(value)
    }
    let value = data
    if (value === null) {
        value = ''
    }
    return (
        <TextField onFocus={() => { ui.hint.set(path) }} onBlur={() => ui.hint.set(null)}
            value={value} rows={3} onChange={onChange} multiline />
    )
}

const BytesTextBox = ({ data, dataUpdate, path }) => {
    const onChange = (e, value) => {
        dataUpdate(value)
    }
    let value = data
    if (value === null) {
        value = ''
    }
    return (
        <TextField
            onFocus={() => { ui.hint.set(path) }} onBlur={() => ui.hint.set(null)}
            value={value} rows={3} onChange={onChange} multiline />
    )
}

const TypeInputBox = ({ type, info, data, dataUpdate, path }) => {
    if (type.type === 'String') {
        return (
            <StringTextBox data={data} dataUpdate={dataUpdate} path={path} />
        )
    } else if (type.type === 'Bytes') {
        return (
            <BytesTextBox data={data} dataUpdate={dataUpdate} path={path} />
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
            <div className='field'>
                <Checkbox className='field-label' label={name} checked={checked} onChange={onCheckedChange} />
                {
                    checked && (
                        <TypeInputBox type={item.ktype} info={info} data={data}
                            dataUpdate={dataUpdate} path={path} />
                    )
                }
            </div>
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
            <div className='field'>
                <div className='field-line'>
                    <Checkbox className='field-label' label={name} checked={checked} onChange={onCheckedChange} />
                    <IconButton iconProps={{ iconName: 'Add' }} onClick={() => onRepeatedAdd()} className='field-icon' />
                </div>
                {
                    checked && items.map((it, index) => (
                        <div className='field-group' key={index}>
                            <div className='field-group-icon'>
                                <IconButton iconProps={{ iconName: 'Clear' }} onClick={() => onRepeatedRemove(index)} className='field-icon' />
                            </div>
                            <div className='field-group-inner'>
                                <TypeInputBox type={item.ktype} info={info} data={it} dataUpdate={data => repeatedUpdate(data, index)}
                                    path={path + '/' + index} />
                            </div>
                        </div>
                    ))
                }
            </div>
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
            {
                m && m.fields.map(item => (
                    <FieldRender key={item.json_name}
                        data={data !== undefined && data !== null ? data[item.json_name] : undefined}
                        dataUpdate={d => update(d, item)} item={item} info={info}
                        path={path + '/' + item.json_name}></FieldRender>
                ))
            }
        </div>
    )
}

const QueryPage = ({ api, tab }) => {
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
                <div><Text variant='large'>.{serviceSelection}.{method}</Text></div>
                <div className='input'>
                    {
                        rpcInfo && (
                            <div>
                                <Toggle checked={editJson} onChange={() => setEditJson(!editJson)} label='Edit json' />
                                <div className={!editJson ? 'show' : 'hide'}>
                                    <MessageRender path='' data={requestData} dataUpdate={data => setRequestData(JSON.parse(JSON.stringify(data)))} message={rpcInfo.request_typename} info={rpcInfo} />
                                </div>
                                <div className={editJson ? 'show' : 'hide'}>
                                    <JsonView message={rpcInfo.request_typename} info={rpcInfo} json={requestData}
                                        jsonUpdate={data => setRequestData(data)} editable />
                                </div>
                                <PrimaryButton onClick={() => onInvokeClick()} className='send-button' text='Send' />
                            </div>
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
                <JsonView json={resultData} />
            </div>
        </div >)
}

export default QueryPage