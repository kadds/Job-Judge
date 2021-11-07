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
    window.localStorage.setItem('history', JSON.stringify(data))
}

function loadAllHistory() {
    const data = window.localStorage.getItem('history')
    if (data === undefined || data === null || data === '') {
        return []
    } else {
        return JSON.parse(data)
    }
}

function clearAllHistory() {
    window.localStorage.setItem('history', [])
}

function loadAllSaved() {
    const data = window.localStorage.getItem('savedItem')
    if (data === undefined || data === null || data === '') {
        return []
    } else {
        return JSON.parse(data)
    }
}

function addSaved(item) {
    let data = loadAllSaved()
    data.push(item)
    window.localStorage.setItem('savedItem', JSON.stringify(data))
}

export {
    historyLimit,
    setHistoryLimit,
    addHistory,
    loadAllHistory,
    clearAllHistory,
    loadAllSaved,
    addSaved,
}