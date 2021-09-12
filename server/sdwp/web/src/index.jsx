import React from 'react';
import ReactDOM from 'react-dom';
import './index.css';
import App from './App';
import { initializeIcons } from '@fluentui/react/lib/Icons';
import { BrowserRouter } from "react-router-dom";


initializeIcons('https://static2.sharepointonline.com/files/fabric/assets/icons/');

ReactDOM.render(
    <React.StrictMode>
        <BrowserRouter>
            <App />
        </BrowserRouter>
    </React.StrictMode>,
    document.getElementById('root')
);