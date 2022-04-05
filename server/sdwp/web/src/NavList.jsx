import React, { useEffect, useState } from 'react'
import { DetailsList, SearchBox, DetailsListLayoutMode, SelectionMode, Text, CheckboxVisibility } from '@fluentui/react'
import { list_service } from './api'
import ui from './store/ui'

const NavList = () => {
    const [items, setItems] = useState([])
    const [finalItems, setFinalItems] = useState([])
    const [update, setUpdate] = useState(0)

    const onSearch = text => {
        setFinalItems(
            items.filter(t => t.toLowerCase().indexOf(text) > -1)
        )
    }
    useEffect(() => {
        let fn = async () => {
            const data = await list_service()
            setItems(data)
            setFinalItems(data)
        }
        fn()
        const id = setInterval(fn, 5000)
        return () => {
            clearInterval(id)
        }
    }, [update])

    const onInvoked = item => {
        const s = item
        if (s != null) {
            ui.tab.add_tab(s)
        }
        setUpdate(update + 1)
    }
    return (
        <div className='navlist'>
            <SearchBox onSearch={onSearch} />
            <DetailsList
                className='detail-list'
                items={finalItems}
                checkboxVisibility={CheckboxVisibility.hidden}
                onItemInvoked={onInvoked}
                selectionPreservedOnEmptyClick={true}
                columns={[{
                    key: 'service',
                    name: 'Service',
                    isIconOnly: false,
                    fieldName: 0,
                    isSizeable: true,
                    onRender: item => (<Text variant='medium'>{item}</Text>)
                }]}
                layoutMode={DetailsListLayoutMode.justified}
                isHeaderVisible={false}
                selectionMode={SelectionMode.single}
            />

        </div>
    )
}

export default NavList