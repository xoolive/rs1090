const socket = new WebSocket('ws://192.168.8.34:5000/websocket');

socket.onopen = function (ev) {
    console.log('connection opened: ', ev)
    // socket.send('Hello Server!');
};

socket.onmessage = function ({data: payload}) {
    let data = JSON.parse(payload)
    console.log('< ', data);
};

socket.onclose = function (ev) {
    console.log('close: %s', ev);
}

socket.onerror = function (ev) {
    console.log('error happens: ', ev)
}

let sendJob = setInterval(() => {
    let datetime = new Date();
    const payload = { payload: "ping",  datetime: datetime.toISOString() };
    // const blob = new Blob([JSON.stringify(obj, null, 2)], {
    //   type: "application/json",
    // });
    let payloadString = JSON.stringify(payload)
    socket.send(payloadString);
    console.log("> ", payloadString);
}, 1000);

// setTimeout(() => {
//     socket.send('About done here...');
//     console.log("Sending close over websocket");
//     socket.close(3000, "Crash and Burn!");
// }, 3000);