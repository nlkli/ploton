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

declare global {
    interface Window {
        ipc: {
            postMessage(message: any): void
        }

        recvIpcMessage: (message: any) => void
        setChartOption: (option: any) => void
    }
}

window.ipc.postMessage("init");

window.recvIpcMessage = (_message) => {
}

window.setChartOption = (option) => {
    chart.setOption(option)
}
