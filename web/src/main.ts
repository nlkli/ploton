import './style.css'
import * as echarts from 'echarts';

document.querySelector<HTMLDivElement>('#app')!.innerHTML = `
<div id="chart"></div>
`

var chartDom = document.getElementById('chart')!;
var chart = echarts.init(chartDom);

window.addEventListener('resize', function () {
    chart.resize();
});

type Event = { type: string, payload: any };

declare global {
    interface Window {
        __PORTAL__: { dispatch(event: Event): void }
    }
}

window.__PORTAL__ = {
    dispatch(event) {
        switch (event.type) {
            case 'setChartOption':
                chart.setOption(event.payload)
                break
        }
    }
}

declare global {
    interface Window {
        ipc: {
            postMessage(message: any): void
        }
    }
}

window.ipc.postMessage("init");
