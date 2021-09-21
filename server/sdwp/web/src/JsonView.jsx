import { TextField } from '@fluentui/react'
import React, { useEffect, useState } from 'react'

const JsonView = ({ json, message, info, jsonUpdate, editable }) => {
    let val = JSON.stringify(json)
    const [input, setInput] = useState(val)

    const onGetErrorMessage = (value) => {
        try {
            const obj = JSON.parse(value)
            jsonUpdate(obj)
        } catch (e) {
            return e + ' '
        }
        return ''
    }

    useEffect(() => {
        if (val !== input) {
            setInput(val)
        }
    }, [json])

    return (
        <div className='json-view'>
            <TextField validateOnLoad={false} rows={10} validateOnFocusOut={true} multiline
                onChange={(e, data) => setInput(data)} value={input} onGetErrorMessage={onGetErrorMessage} />
        </div>
    )
}

export default JsonView