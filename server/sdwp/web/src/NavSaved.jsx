import React, { useEffect, useState } from 'react'
import { DetailsList, Selection, DetailsListLayoutMode, SelectionMode, Text, CheckboxVisibility, Stack, FontIcon, Dialog, BaseButton, PrimaryButton, DefaultButton, Callout, DirectionalHint } from '@fluentui/react'
import { loadAllSaved, delSaved } from './config'
import { inject, observer } from 'mobx-react'

const NavSaved = inject('store')(observer(({ store }) => {
    const [items, setItems] = useState([])
    const [calloutTarget, setCalloutTarget] = useState(null)

    useEffect(() => {
        let data = loadAllSaved()
        data.sort((a, b) => b.time - a.time)
        setItems(data)
    }, [store.ui.dataVersion.saved])
    const onInvoked = item => {
        store.ui.tab.add_tab_with(item.module, item)
    }
    const selection = new Selection({
        onSelectionChanged: () => console.log(selection.getSelection()[0])
    })
    const onCloseClick = item => {
        delSaved(item)
        store.ui.dataVersion.notify_saved()
        setCalloutTarget(null)
    }
    const onCloseRequestClick = (e, item) => {
        setCalloutTarget({ item, ele: e.nativeEvent })
    }

    const ItemRender = ({ item }) => {
        let date = new Date()
        date.setTime(item.time * 1000)
        return (
            <Stack tokens={{ childrenGap: 4 }} horizontal horizontalAlign='space-between' verticalAlign='center'>
                <Stack>
                    <Text variant='mediumPlus' >{`[${item.module}] ${item.method}`}</Text>
                    <Text variant='xSmall'>{date.toLocaleString()}</Text>
                </Stack>
                <FontIcon onClick={e => onCloseRequestClick(e, item)} className='icon' aria-label='Close' iconName='ChromeClose'></FontIcon>
            </Stack>
        )
    }
    return (
        <div className='navlist'>
            <DetailsList
                className='detail-list'
                items={items}
                checkboxVisibility={CheckboxVisibility.hidden}
                onItemInvoked={onInvoked}
                selectionPreservedOnEmptyClick={true}
                selection={selection}
                columns={[{
                    key: 'id',
                    name: 'Saved',
                    isIconOnly: false,
                    fieldName: 0,
                    isSizeable: false,
                    onRender: (item) => (<ItemRender item={item}></ItemRender>),
                }]}
                setKey='id'
                layoutMode={DetailsListLayoutMode.justified}
                isHeaderVisible={true}
                selectionMode={SelectionMode.single}
            />
            {
                calloutTarget && (
                    <Callout
                        role="dialog"
                        setInitialFocus
                        target={calloutTarget.ele}
                        directionalFixed
                        directionalHint={DirectionalHint.rightCenter}
                        style={{ padding: '20px 24px' }}
                        onDismiss={() => setCalloutTarget(null)}
                    >
                        <Text variant='mediumPlus'>Do you want to delete this record?</Text>
                        <Stack style={{ marginTop: 14 }} horizontal horizontalAlign='space-between'>
                            <PrimaryButton text='Cancel' onClick={() => setCalloutTarget(null)}></PrimaryButton>
                            <DefaultButton text='Delete' onClick={() => onCloseClick(calloutTarget.item)}></DefaultButton>
                        </Stack>
                    </Callout>
                )
            }

        </div>
    )
}))

export default NavSaved