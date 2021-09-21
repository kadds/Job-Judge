import React, { useEffect, useState } from 'react'
import { DetailsList, Selection, SearchBox, DetailsListLayoutMode, SelectionMode, Label, Text, CheckboxVisibility } from '@fluentui/react'
import { loadAllHistory } from './config'

const NavHistory = () => {
    const [items, setItems] = useState([])
    const [finalItems, setFinalItems] = useState([])

    const onSearch = text => {
        setFinalItems(
            items.filter(t => t.toLowerCase().indexOf(text) > -1)
        )
    }
    useEffect(() => {
        const data = loadAllHistory()
        setItems(data)
        setFinalItems(data)
    }, [])
    const onInvoked = item => {
    }
    const selection = new Selection({
        onSelectionChanged: () => console.log(selection.getSelection()[0])
    })
    return (
        <div className='navlist'>
            <SearchBox onSearch={onSearch} />
            <DetailsList
                className='detail-list'
                items={finalItems}
                checkboxVisibility={CheckboxVisibility.hidden}
                onItemInvoked={onInvoked}
                selectionPreservedOnEmptyClick={true}
                selection={selection}
                columns={[{
                    key: 'history',
                    name: 'History',
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
    )
}

export default NavHistory