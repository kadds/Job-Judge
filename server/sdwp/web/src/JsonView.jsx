import { IconButton, Stack, TextField } from '@fluentui/react'
import React, { useEffect, useState } from 'react'

const FieldView = ({ name, value, comma }) => {
    if (value === undefined || value === null) {
        return <div></div>
    }
    else if (Array.isArray(value)) {
        return (
            <div className='view-field field-vertical'>
                <span className='view-field-key json-text'>{name ? `"${name}": [` : '['}</span>
                <div className='view-field-value field-indent'>
                    <ArrayView value={value} />
                </div>
                <div className='view-field-bottom json-text'>
                    <span className='json-text'>{']'}</span>
                    {comma && <span className='json-text'>{','}</span>}
                </div>
            </div>
        )
    } else if (typeof value === 'object' && value !== null) {
        return (
            <div className='view-field field-vertical'>
                <span className='view-field-key json-text'>{name ? `"${name}": {` : '{'}</span>
                <div className='view-field-value field-indent'>
                    <ObjectView value={value} />
                </div>
                <div className='view-field-bottom'>
                    <span className='json-text'>{'}'}</span>
                    {comma && <span className='json-text'>{','}</span>}
                </div>
            </div>
        )
    } else {
        const str = typeof value === 'string'
        if (value === null) {
            value = "null"
        }
        return (
            <div className='view-field'>
                <span className='view-field-key json-text'>{name ? `"${name}": ` : ''}</span>
                <span className='view-field-value json-text'>{str ? `"${value.toString()}"` : value.toString()}</span>
                {comma && <span className='json-text'>{','}</span>}
            </div>
        )
    }
}

const ObjectView = ({ value }) => {
    if (value === null) {
        return null
    } else {
        const obj = Object.entries(value)
        return (<div>
            {obj.map((item, index) => (
                <div key={item[0]}>
                    <FieldView name={item[0]} value={item[1]} comma={index + 1 < obj.length} />
                </div>))}
        </div>)
    }
}

const ArrayView = ({ value }) => {
    return (<div>
        {
            value.map((item, index) => (
                <FieldView key={index} value={item} comma={index + 1 < value.length}></FieldView>
            ))
        }
    </div>)
}
const JsonView = ({ object, message, info, objectUpdate }) => {
    const parseClick = async () => {
        const text = await navigator.clipboard.readText()
        const obj = JSON.parse(text)
        if (objectUpdate) {
            objectUpdate(obj)
        }
    }
    return (
        <div className='json-view'>
            <Stack horizontal horizontalAlign='flex-end'>
                <IconButton iconProps={{ iconName: 'Copy' }} onClick={() => navigator.clipboard.writeText(JSON.stringify(object))} />
                {
                    objectUpdate && (
                        <IconButton iconProps={{ iconName: 'FileTemplate' }} onClick={parseClick} />
                    )
                }
            </Stack>
            <FieldView value={object} comma={false} />
        </div>
    )
}

export default JsonView