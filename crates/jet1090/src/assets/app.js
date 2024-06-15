// channel https://hexdocs.pm/phenox/js/

const { Socket, Channel, Presence  } = Phoenix;
import morphdom from 'https://unpkg.com/morphdom@2.6.1/dist/morphdom-esm.js';

// https://www.npmjs.com/package/morphdom#morphdomfromnode-tonode-options--node
console.dir(morphdom);


let userToken = 'userSocketToken';
let debug = true;
let socket = new Socket("", {debug: false, params: {userToken}});
socket.connect();

let systemChannel = 'system';
const systemChanelToken = 'channel-token';
let channel = socket.channel(systemChannel, {token: systemChanelToken});

channel.on('datetime', ({status, response: {datetime, counter}})=> {
  console.log(`status: ${status}, datetime, ${datetime}, counter: ${counter}`); 

  document.getElementById('datetime').innerText = datetime;
  document.getElementById('counter').innerHTML = counter;
})

channel
    .join()
    .receive('ok', (ev) => {
        console.log(`(${systemChannel} - joined`, ev)
    })
    .receive('error', ({reason}) => {
        console.log(`${systemChannel} - failed to join`, reason);
    })
    .receive('timeout', () => {
        console.log(`${systemChannel} - joining timeouts`);
    });
