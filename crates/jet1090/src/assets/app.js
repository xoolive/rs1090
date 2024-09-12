// channel https://hexdocs.pm/phenox/js/

const { Socket, Channel, Presence } = Phoenix;
import morphdom from 'https://unpkg.com/morphdom@2.6.1/dist/morphdom-esm.js';

// https://www.npmjs.com/package/morphdom#morphdomfromnode-tonode-options--node
console.dir(morphdom);


let userToken = 'userSocketToken';
let debug = true;
let socket = new Socket("", { debug: false, params: { userToken } });
socket.connect();

let systemChannelName = 'system';
const systemChanelToken = 'channel-token';
let systemChannel = socket.channel(systemChannelName, { token: systemChanelToken });

systemChannel.on('datetime', ({ status, response: { datetime, counter } }) => {
  // console.log(`status: ${status}, datetime, ${datetime}, counter: ${counter}`);

  document.getElementById('datetime').innerText = datetime;
  document.getElementById('counter').innerHTML = counter;
})

systemChannel
  .join()
  .receive('ok', (ev) => {
    console.log(`(${systemChannel} - joined`, ev)
  })
  .receive('error', ({ reason }) => {
    console.log(`${systemChannel} - failed to join`, reason);
  })
  .receive('timeout', () => {
    console.log(`${systemChannel} - joining timeouts`);
  });

/// jet1090 channel
let jet1090ChannelName = 'jet1090';
let jet1090Channel = socket.channel(jet1090ChannelName, { token: 'jet1090-token' });
jet1090Channel.on('data', ({ status, response: { timed_message: { timestamp } } }) => {
  document.getElementById('jet1090-latest-ts').innerHTML = timestamp;
})

jet1090Channel
  .join()
  .receive('ok', (ev) => {
    console.log(`(${jet1090Channel} - joined`, ev)
  })
  .receive('error', ({ reason }) => {
    console.log(`${jet1090Channel} - failed to join`, reason);
  })
  .receive('timeout', () => {
    console.log(`${jet1090Channel} - joining timeouts`);
  });


