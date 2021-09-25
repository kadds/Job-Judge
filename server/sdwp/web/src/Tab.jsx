import { FontIcon, Text, ProgressIndicator, ContextualMenu } from '@fluentui/react'
import React, { useEffect, useRef, useState } from 'react'
import { motion, AnimatePresence } from "framer-motion"


const menuItems = [
    { key: 'close', text: 'Close current tab' },
    { key: 'close_all', text: 'Close all' },
    { key: 'close_other', text: 'Close others' }
]
const tab_variants = {
    initial: {
        y: -50,
        opacity: 0,
        scale: 1,
    },
    animate: {
        y: 0,
        opacity: 1,
        scale: 1,
    },
    exit: {
        y: 0,
        opacity: 0,
        scale: 0.3,
    },
    whileTap: {
        y: 0,
        opacity: 1,
        scale: 1.05,
    }
}

const Tab = props => {
    const ref = useRef(null)
    const [showMenu, setShowMenu] = useState(false)
    const [targetMenu, setTargetMenu] = useState(null)

    const onClick = tab => {
        props.onSelect(tab)
    }
    const onCloseClick = (e, tab) => {
        e.stopPropagation()
        props.onClose(tab)
    }
    const onMenuClick = (e, m) => {
        if (m.key === 'close') {
            props.onClose(props.tabs.find(item => item.id == props.select))
        } else if (m.key === 'close_all') {
            props.tabs.map(item => props.onClose(item))
        } else if (m.key === 'close_other') {
            props.tabs.map(item => { if (item.id !== props.select) props.onClose(item) })
        }
    }
    const handleContextMenu = e => {
        setTargetMenu(e)
        e.preventDefault()
        setShowMenu(true)
    }
    useEffect(() => {
        ref.current.addEventListener("contextmenu", handleContextMenu)
        return () => {
            ref.current.removeEventListener("contextmenu", handleContextMenu)
        }
    })
    return (
        <div className='tab-root' ref={ref}>
            <AnimatePresence>
                {
                    props.tabs.map(tab => (
                        <motion.div
                            {...tab_variants}
                            className={'tab-element-wrapper ' + ((props.select === tab.id) ? 'select' : '')}
                            key={tab.id} onClick={() => onClick(tab)}>
                            <Text className='tab-element'>{tab.name}</Text>
                            {
                                tab.loading && (
                                    <ProgressIndicator className='tab-bottom-element'></ProgressIndicator>
                                )
                            }
                            <FontIcon onClick={e => onCloseClick(e, tab)} className='tab-element icon' aria-label='Close' iconName='ChromeClose'></FontIcon>
                        </motion.div>
                    ))
                }
            </AnimatePresence>
            <ContextualMenu
                items={menuItems}
                hidden={!showMenu}
                target={targetMenu}
                onDismiss={() => setShowMenu(false)}
                onItemClick={onMenuClick}
            />
        </div>
    )
}

export default Tab