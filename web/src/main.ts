import './style.css'
import * as echarts from 'echarts'

type PortalEvent = { type: string; payload: any }

declare global {
    interface Window {
        // Provided by the native host (wry).
        ipc: { postMessage(message: string): void }
        // Entry point for messages from the native host.
        __PORTAL__: { dispatch(event: PortalEvent): void }
    }
}

document.querySelector<HTMLDivElement>('#app')!.innerHTML = '<div id="chart"></div>'

const chart = echarts.init(document.getElementById('chart')!, 'vintage')
window.addEventListener('resize', () => chart.resize())

window.__PORTAL__ = {
    dispatch(event) {
        switch (event.type) {
            case 'setChartOption':
                chart.setOption(event.payload)
                break
        }
    },
}

// Tell the host the page is ready to receive options.
window.ipc.postMessage('init')
