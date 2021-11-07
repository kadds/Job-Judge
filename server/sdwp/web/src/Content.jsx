import React from 'react'
import Tab from './Tab'
import { inject, observer } from 'mobx-react'
import { motion, AnimatePresence } from "framer-motion"
import QueryPage from './QueryPage'

const variants = {
    show: {
        opacity: 1,
        scale: 1,
        display: 'block'
    },
    hide: {
        opacity: 0,
        scale: 1,
        transitionEnd: {
            display: 'none'
        }
    }
}

const Content = inject('store')(observer(props => {
    let tab_ui = props.store.ui.tab
    const onSelect = t => {
        tab_ui.select_tab(t)
    }

    const onClose = t => {
        tab_ui.close_tab(t)
    }

    return (
        <div className='main-content'>
            <Tab tabs={tab_ui.tabs.slice()} select={tab_ui.selected}
                onSelect={onSelect} onClose={onClose}>

            </Tab>
            <div className='content'>
                <AnimatePresence>
                    {
                        tab_ui.tabs.slice().map(tab => (
                            <motion.div key={tab.id}
                                initial='hide'
                                animate={tab.id === tab_ui.selected ? 'show' : 'hide'}
                                variants={variants}
                                className={'content-inner ' + (tab.id === tab_ui.selected ? 'select' : '')}>
                                <QueryPage tab={tab} api={tab.name} init={tab.init} />
                            </motion.div>
                        ))
                    }
                </AnimatePresence>
            </div>
        </div>
    )
}))


export default Content