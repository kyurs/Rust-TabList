if (!process.env.PROBE_HOST) throw new Error('Set PROBE_HOST to an authorized test server');
const mc=require('minecraft-protocol');
const assert=require('node:assert/strict');
const username=process.env.PROBE_NAME||'TabListProbe';
const c=mc.createClient({host:process.env.PROBE_HOST,port:Number(process.env.PROBE_PORT||25565),username,auth:'offline',version:'1.12.2'});
const packets=[];
const plain=o=>typeof o==='string'?o:Array.isArray(o)?o.map(plain).join(''):(o?.text||o?.translate||'')+(o?.with||[]).map(plain).join('')+(o?.extra||[]).map(plain).join('');
let header=false,member=false,chat=false,denied=false;
const timeout=setTimeout(()=>{c.end();console.error('Timed out', {header,member,chat,denied});process.exitCode=1;},45000);
c.on('error',e=>{console.error(e);process.exitCode=1;});
c.on('packet',(p,m)=>{
 if(m.name==='position') c.write('teleport_confirm',{teleportId:p.teleportId});
 if(m.name==='playerlist_header') {const h=plain(JSON.parse(p.header)),f=plain(JSON.parse(p.footer)); header=h.includes('Rust Tab List')&&h.includes('Have Fun')&&/Players .*TPS .*MSPT/.test(f); if(header&&!packets.some(x=>x.type==='header')){packets.push({type:'header',header:h,footer:f}); console.log('HEADER',h,'FOOTER',f);}}
 if(m.name==='player_info'&&p.action==='update_display_name') for(const item of p.data) if(item.displayName){const name=plain(JSON.parse(item.displayName));if(name.includes(username)&&name.includes('[Member]'))member=true;packets.push({type:'name',name,raw:item.displayName});}
 if(m.name==='chat') {const raw=JSON.parse(p.message),s=plain(raw);console.log('CHAT',s);packets.push({type:'chat',text:s,raw}); if(s.includes('[Member] '+username+' » tablist smoke test'))chat=true; if(/permission|Unknown command|Incorrect argument|Unknown or incomplete/i.test(s))denied=true;}
});
c.once('login',()=>{
 c.write('settings',{locale:'en_US',viewDistance:2,chatFlags:0,chatColors:true,skinParts:127,mainHand:1});
 setTimeout(()=>c.write('chat',{message:'tablist smoke test'}),2500);
 setTimeout(()=>c.write('chat',{message:'/tablist ranks color member #FF0000'}),4000);
 setTimeout(()=>{clearTimeout(timeout);c.end();require('node:fs').writeFileSync('last-probe.json',JSON.stringify(packets,null,2));try {assert(header,'header/footer');assert(member,'member tab prefix');assert(chat,'ranked chat');assert(denied,'non-op blocked');console.log('PASS: tab, metrics, chat, permission boundary');}catch(e){console.error(e);process.exitCode=1;}},25000);
});
c.on('kick_disconnect',p=>console.error('KICK',p));
