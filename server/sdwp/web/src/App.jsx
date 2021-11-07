import React, { useState } from 'react'
import NavList from './NavList'
import NavHistory from './NavHistory'
import NavSaved from './NavSaved'
import Content from './Content'
import Panel from './Panel'
import Setting from './Setting'
import { DefaultButton, Dialog, DefaultEffects, DialogFooter, DialogType, MessageBar, MessageBarType, PrimaryButton, Separator, TextField, Text, IconButton } from '@fluentui/react'
import { login } from './api'
import { inject, observer } from 'mobx-react'
import { motion, AnimatePresence } from "framer-motion"

const menu = [
    { name: 'Services', icon: 'List', render: <NavList /> },
    { name: 'History', icon: 'Recent', render: <NavHistory /> },
    { name: 'Saved', icon: 'FabricOpenFolderHorizontal', render: <NavSaved /> }
]

const bottom_menu = [
    { name: 'Setting', icon: 'Settings', render: <Setting /> }
]

const message_bar_variants = {
    initial: {
        x: 0,
        y: -50,
        opacity: 0,
    },
    animate: {
        x: 0,
        y: 0,
        opacity: 1,
    },
    exit: {
        x: 20,
        y: 0,
        opacity: 0,
    }
}

const hint_bar_variants = {
    initial: {
        x: 0,
        y: -50,
        opacity: 0,
    },
    animate: {
        x: 0,
        y: 0,
        opacity: 1,
    },
    exit: {
        x: 0,
        y: -50,
        opacity: 0,
    }
}

const App = inject('store')(observer(props => {
    const ui = props.store.ui
    const [loginData, setLoginData] = useState({ username: '', password: '' })
    const [copyDisabled, setCopyDisabled] = useState(false)
    const resetClick = () => {
        setLoginData({ username: '', password: '' })
    }

    const loginClick = () => {
        (async () => {
            await login(loginData.username, loginData.password)
            ui.login.hide_dialog()
            setTimeout(() => {
                window.location.reload()
            }, 1000)
        })()
    }
    const onCopy = () => {
        navigator.clipboard.writeText(ui.hint.text)
        setCopyDisabled(true)
        setTimeout(() => {
            setCopyDisabled(false)
        }, 2000)
    }
    return (
        <div className="app">
            <Panel headerText={'Service Debug Web Page'} menu={menu} bottom_menu={bottom_menu}>
            </Panel>
            <Separator vertical={true} />
            <Content />
            <div className='float-window'>
                <AnimatePresence>
                    {
                        ui.errors.text.slice().map(item => (
                            <motion.div
                                {...message_bar_variants}
                                onMouseEnter={() => ui.errors.keep(item)}
                                onMouseLeave={() => ui.errors.new_timer(item)}
                                className={'error-message-bar'}
                                style={{ boxShadow: DefaultEffects.elevation16 }}
                                key={item.id}>
                                <MessageBar
                                    messageBarType={MessageBarType.error}
                                    isMultiline={true}>
                                    <div> {item.status + '  ' + item.statusText} </div>
                                    <div style={{ marginLeft: '8px' }}>
                                        at {item.url}
                                    </div>
                                    <div>
                                        {item.data}
                                    </div>
                                </MessageBar>
                            </motion.div>
                        ))
                    }
                </AnimatePresence>
            </div>

            <AnimatePresence>
                {
                    ui.hint.show && (<motion.div
                        {...hint_bar_variants}
                        className='hint-window ' style={{ boxShadow: DefaultEffects.elevation16 }}>
                        <Text>{ui.hint.text}</Text>
                        <IconButton onClick={onCopy} disabled={copyDisabled} iconProps={{ iconName: 'Copy' }} />
                    </motion.div>)
                }
            </AnimatePresence>

            <Dialog
                hidden={!ui.login.show}
                onDismiss={() => ui.login.hide_dialog()}
                dialogContentProps={{
                    type: DialogType.normal,
                    title: 'Login',
                }}
            >
                <div>
                    <TextField label='Username' value={loginData.username}
                        onChange={(e, value) => setLoginData({ ...loginData, username: value })} />
                    <TextField label='Password' type='password' canRevealPassword value={loginData.password}
                        onChange={(e, value) => setLoginData({ ...loginData, password: value })} />
                </div>
                <DialogFooter>
                    <PrimaryButton onClick={loginClick} text='Login' />
                    <DefaultButton onClick={resetClick} text='Reset' />
                </DialogFooter>
            </Dialog>
        </div>
    )
}))

export default App