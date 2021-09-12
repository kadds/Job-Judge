import React, { useEffect, useState } from 'react';
import { Text, Separator, Slider, DefaultButton, Label } from '@fluentui/react';
import { clearAllHistory, historyLimit, loadAllHistory, setHistoryLimit } from './config'

const Setting = () => {
    const [historyValue, setHistoryValue] = useState(0)
    const [historySize, setHistorySize] = useState(0)
    const onHistoryValueChange = value => {
        if (value === undefined) {
            return;
        }
        setHistoryValue(value)
        setHistoryLimit(value)
    }
    useEffect(() => {
        setHistoryValue(historyLimit())
        setHistorySize(loadAllHistory().length)
    }, [])

    const onClearClick = () => {
        clearAllHistory()
        setHistoryValue(historyLimit())
        setHistorySize(loadAllHistory().length)
    }
    return (<div>
        <Text>Settings</Text>
        <Separator />
        <Slider label="History Limit"
            max={10000} min={0} step={500}
            value={historyValue}
            onChange={onHistoryValueChange}>

        </Slider>
        <Separator />
        <Label>Current history size: {historySize}</Label>
        <Separator />
        <DefaultButton onClick={onClearClick} text='Clear History'></DefaultButton>
    </div>)
}

export default Setting;