import { IconButton, TextField } from '@fluentui/react'
import React, { useEffect, useState } from 'react'

const FieldView = ({ name, value, comma }) => {
    if (value === undefined) {
        return null
    }
    else if (Array.isArray(value)) {
        return (
            <div className='view-field field-vertical'>
                <div className='view-field-key'>{name ? `"${name}" : [` : '['}</div>
                <div className='view-field-value'>
                    <ArrayView value={value} />
                </div>
                <div className='view-field-bottom'>
                    <span>{']'}</span>
                    {comma && <span>{','}</span>}
                </div>
            </div>
        )
    } else if (typeof value === 'object' && value !== null) {
        return (
            <div className='view-field field-vertical'>
                <div className='view-field-key'>{name ? `"${name}" : {` : '{'}</div>
                <div className='view-field-value'>
                    <ObjectView value={value} />
                </div>
                <div className='view-field-bottom'>
                    <span>{'}'}</span>
                    {comma && <span>{','}</span>}
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
                <div className='view-field-key'>{name ? `"${name}" : ` : ''}</div>
                <div className='view-field-value'>{str ? `"${value.toString()}"` : value.toString()}</div>
                {comma && <div>{','}</div>}
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
                <FieldView value={item} comma={index + 1 < value.length}></FieldView>
            ))
        }
    </div>)
}

const JsonView = ({ object, message, info }) => {
    return (
        <div className='json-view'>
            {/* <TextField validateOnLoad={false} rows={10} validateOnFocusOut={true} multiline
                onChange={(e, data) => setInput(data)} value={input} onGetErrorMessage={onGetErrorMessage} /> */}
            <div>
                <IconButton iconProps={{ iconName: 'copy' }} />
            </div>
            <FieldView value={object} comma={false} />
        </div>
    )
}

export default JsonView