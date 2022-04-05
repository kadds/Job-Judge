import { IconButton, Text, Separator } from '@fluentui/react'
import React, { useState, useEffect } from 'react'
import { motion, AnimatePresence, m } from "framer-motion"

const itemHoverVariants = {
    scale: 1.1,
}

const itemTapVariants = {
    scale: 1.4,
}

const sidebar = {
    open: {
        width: 240,
        transition: {
            type: "spring",
            stiffness: 150,
        }
    },
    closed: {
        width: 30,
        transition: {
            type: "spring",
            stiffness: 150,
        }
    }
}

const sidebar_mask = {
    open: {
        clipPath: `ellipse(300px 100% at 0% 50%)`,
        transition: {
            type: "spring",
            stiffness: 30,
        }
    },
    closed: {
        clipPath: `ellipse(60px 100% at 0% 50%)`,
        transition: {
            type: "spring",
            stiffness: 80,
        }
    }
}

const Panel = props => {
    let [open, setOpen] = useState(true)
    let [newMenus, setNewMenus] = useState([])
    const onClick = () => {
        setOpen(!open)
    }
    const onMenuClick = menu => {
        let item = props.menu.concat(props.bottom_menu).find(v => v.name === menu.name)
        if (item) {
            item.select = true
            setNewMenus([item])
        }
    }
    useEffect(() => {
        let item = props.menu.find(v => v.select)
        if (item) {
            setNewMenus([item])
        }
    }, [])


    return (
        <motion.div layout animate={open ? "open" : "closed"} className={'panel'} variants={sidebar}>
            <motion.div className='panel-inner' variants={sidebar_mask}>
                <div className='header'>
                    <motion.div whileHover={itemHoverVariants} whileTap={itemTapVariants}>
                        <IconButton onClick={onClick} title='Expand/Sink' iconProps={{ iconName: 'GlobalNavButton' }}></IconButton>
                    </motion.div>
                    <Text className='title' variant='mediumPlus' block={true}>{props.headerText}</Text>
                </div>
                <Separator />
                <div className='panel-content'>
                    <div className='icons-content'>
                        <div className='icons-front-content'>
                            {
                                props.menu.map(menu => (
                                    <motion.div key={menu.name} whileHover={itemHoverVariants} whileTap={itemTapVariants}>
                                        <IconButton onClick={() => onMenuClick(menu)} title={menu.name} iconProps={{ iconName: menu.icon }}></IconButton>
                                    </motion.div>
                                ))
                            }
                        </div>
                        <div className='icons-back-content'>
                            {
                                props.bottom_menu.map(menu => (
                                    <motion.div key={menu.name} whileHover={itemHoverVariants} whileTap={itemTapVariants}>
                                        <IconButton onClick={() => onMenuClick(menu)} title={menu.name} iconProps={{ iconName: menu.icon }}></IconButton>
                                    </motion.div>
                                ))
                            }
                        </div>
                    </div>
                    <div className='menu-content'>
                        <AnimatePresence>
                            {
                                newMenus.map(menu => (
                                    menu.select && (
                                        <motion.div className='content-wrapper' key={menu.name}
                                            initial={{ y: -50, opacity: 0, scale: 1 }}
                                            animate={{ y: 0, opacity: 1, scale: 1 }}
                                            exit={{ y: 200, opacity: 0, scale: 0.5 }}>
                                            {menu.render}
                                        </motion.div>
                                    )
                                ))
                            }
                        </AnimatePresence>
                    </div>
                </div>
            </motion.div>
        </motion.div >
    )
}

export default Panel