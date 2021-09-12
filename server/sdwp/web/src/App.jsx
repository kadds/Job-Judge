import React, { useState } from 'react';
import NavList from './NavList';
import NavHistory from './NavHistory';
import Content from './Content';
import Panel from './Panel';
import Setting from './Setting';
import { Separator } from '@fluentui/react';

const menu = [
    { name: 'Services', icon: 'List', render: <NavList /> },
    { name: 'History', icon: 'Recent', render: <NavHistory /> },
]

const bottom_menu = [
    { name: 'Setting', icon: 'Settings', render: <Setting /> }
]

const App = () => {

    return (
        <div className="app">
            <Panel headerText={'Service Debug Web Page'} menu={menu} bottom_menu={bottom_menu}>
            </Panel>
            <Separator vertical={true} />
            <Content>

            </Content>
        </div>
    );
};

export default App;