import React, { useEffect, useState } from 'react'
import { DetailsList, Selection, DetailsListLayoutMode, SelectionMode, Text, CheckboxVisibility, Stack } from '@fluentui/react'
import { loadAllSaved } from './config'
import { inject, observer } from 'mobx-react'

const NavSaved = inject('store')(observer(({ store }) => {
    const [items, setItems] = useState([])

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
    const ItemRender = ({ item }) => {
        let date = new Date()
        date.setTime(item.time * 1000)
        return (
            <Stack>
                <Text variant='mediumPlus' >{`[${item.module}] ${item.method}`}</Text>
                <Text variant='xSmall'>{date.toLocaleString()}</Text>
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

        </div>
    )
}))

export default NavSaved