import { observable, action } from 'mobx'

let id = 0

let tab = observable({
    tabs: [],
    selected: -1,
    add_tab: action(name => {
        id += 1
        tab.tabs.push({ id, name, loading: true })
        tab.selected = id
    }),
    close_tab: action(t => {
        const tab_index = tab.tabs.findIndex(item => item.id === t.id)
        if (tab_index >= 0) {
            tab.tabs.remove(tab.tabs[tab_index])
            if (t.id === tab.selected) {
                if (tab.tabs.length > 0) {
                    let index = Math.max(tab_index - 1, 0)
                    index = Math.min(index, tab.tabs.length - 1)
                    tab.selected = tab.tabs[index].id
                } else {
                    tab.selected = -1
                }
            }
        }
    }),
    select_tab: action(t => {
        tab.selected = t.id
    }),
    finish_loading: action(t => {
        const obj_index = tab.tabs.findIndex(item => item.id === t.id)
        if (obj_index >= 0) {
            let obj = { ...tab.tabs[obj_index], loading: false }
            tab.tabs[obj_index] = obj
        }
    }),
    loading_tab: action(t => {
        const obj_index = tab.tabs.findIndex(item => item.id === t.id)
        if (obj_index >= 0) {
            let obj = { ...tab.tabs[obj_index], loading: true }
            tab.tabs[obj_index] = obj
        }
    })
})

let login = observable({
    show: false,
    show_dialog: action(() => {
        login.show = true
    }),
    hide_dialog: action(() => {
        login.show = false
    })
})

let errors = observable({
    text: [],
    push: action((url, status, statusText, data) => {
        id++
        const cid = id
        const tid = setTimeout(() => {
            errors.pop_id(cid)
        }, 4 * 1000)
        errors.text.push({ id, url, status, statusText, data, tid })
    }),
    pop_id: action(id => {
        errors.text.remove(errors.text.find(item => item.id === id))
    }),
    pop: action(() => {
        errors.text.shift()
    }),
    keep: action((it) => {
        const obj_index = errors.text.findIndex(item => item.id === it.id)
        if (obj_index >= 0) {
            let obj = errors.text[obj_index]
            if (obj.tid !== null) {
                clearTimeout(obj.tid)
                errors.text[obj_index] = { ...obj, tid: null }
            }
        }
    }),
    new_timer: action((it) => {
        const obj_index = errors.text.findIndex(item => item.id === it.id)
        if (obj_index >= 0) {
            let obj = errors.text[obj_index]
            if (obj.tid === null) {
                const tid = setTimeout(() => {
                    errors.pop_id(obj.id)
                }, 4 * 1000)
                errors.text[obj_index] = { ...obj, tid }
            }
        }
    })
})

let hint_timer = null

let hint = observable({
    text: '',
    show: false,
    set: action(text => {
        if (text === null || text === '') {
            if (hint_timer !== null) {
                clearTimeout(hint_timer)
                hint_timer = null
            }
            hint_timer = setTimeout(() => {
                hint.hide_window()
                hint_timer = null
            }, 1000)
        } else {
            hint.text = text
            hint.show = true
            if (hint_timer !== null) {
                clearTimeout(hint_timer)
                hint_timer = null
            }
        }
    }),
    show_window: action(() => {
        hint.show = true
    }),
    hide_window: action(() => {
        hint.show = false
    })
})

const ui = { tab, login, errors, hint }

export default ui
