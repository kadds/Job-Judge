function historyLimit() {
    return window.localStorage.getItem('historyLimit') || 1000
}

function setHistoryLimit(limit) {
    window.localStorage.setItem('historyLimit', limit)
}

function addHistory(item) {
    let data = loadAllHistory()
    if (data.length > historyLimit()) {
        data = data.shift()
    }
    data.push(item)
    window.localStorage.setItem('history', data)
}

function loadAllHistory() {
    const data = window.localStorage.getItem('history')
    if (data === undefined || data === null || data === '') {
        return []
    } else {
        return data
    }
}

function clearAllHistory() {
    window.localStorage.setItem('history', [])
}

export {
    historyLimit,
    setHistoryLimit,
    addHistory,
    loadAllHistory,
    clearAllHistory,
}