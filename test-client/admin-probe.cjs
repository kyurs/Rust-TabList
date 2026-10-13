if (!process.env.PROBE_HOST) throw new Error('Set PROBE_HOST to an authorized test server');
const mc=require('minecraft-protocol');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const plain=o=>typeof o==='string'?o:Array.isArray(o)?o.map(plain).join(''):(o?.text||o?.translate||'')+(o?.with||[]).map(plain).join('')+(o?.extra||[]).map(plain).join('');
const c=mc.createClient({host:process.env.PROBE_HOST,port:25565,username:'TabAdminProbe',auth:'offline',version:'1.12.2'});
let ready;const readiness=new Promise(r=>ready=r);const colors=new Set();const log=[];let pending;
const hard=setTimeout(()=>{c.end();process.exitCode=1;console.error('Admin test timed out');},180000);
c.on('error',console.error);
c.on('packet',(p,m)=>{
 if(m.name==='position')c.write('teleport_confirm',{teleportId:p.teleportId});
 if(m.name==='playerlist_header'&&plain(JSON.parse(p.header)).includes('KYURS'))ready();
 if(m.name==='player_info'&&p.action==='update_display_name')for(const row of p.data){if(row.displayName&&plain(JSON.parse(row.displayName)).includes('[Owner] TabAdminProbe'))colors.add(row.displayName);}
 if(m.name==='chat'){const s=plain(JSON.parse(p.message));log.push(s);console.log(new Date().toISOString(),s.slice(0,130));if(pending&&pending.pattern.test(s)){const {resolve}=pending;pending=undefined;resolve(s);} }
});
c.once('login',()=>c.write('settings',{locale:'en_US',viewDistance:2,chatFlags:0,chatColors:true,skinParts:127,mainHand:1}));
function command(s,pattern){return new Promise((resolve,reject)=>{const t=setTimeout(()=>{pending=undefined;reject(new Error('No expected response: '+s));},20000);pending={pattern,resolve:x=>{clearTimeout(t);resolve(x)}};c.write('chat',{message:s});});}
(async()=>{
try{
 await readiness;
 await command('/tablist ranks',/Owner.*Rainbow/);
 await command('/tablist ranks gradient member #55FF55 #AAFFAA',/Rank settings saved/);
 await command('/tablist ranks reload',/Rank settings reloaded/);
 await command('/tablist ranks',/member: Member.*Gradient/);
 await command('/tablist ranks color member #55FF55',/Rank settings saved/);
 await command('/tablist ranks permission member pumpkin:command.tps allow',/Rank settings saved/);
 await command('/tablist ranks',/pumpkin:command.tps.*true/);
 await command('/tablist ranks permission member pumpkin:command.tps unset',/Rank settings saved/);
 assert(colors.size>1,'Owner rainbow updates');
 console.log('PASS: Owner access, animated rainbow, gradient, disk reload, permissions, default restoration');
}catch(e){console.error(e);process.exitCode=1;
 try{await command('/tablist ranks color member #55FF55',/Rank settings saved/);await command('/tablist ranks permission member pumpkin:command.tps unset',/Rank settings saved/);}catch(restore){console.error('RESTORE NEEDED',restore);}
}finally{try{await command('/deop TabAdminProbe',/deop|operator|commands/);}catch(e){console.error('CHECK TEMP OP CLEANUP',e);process.exitCode=1;}fs.writeFileSync('last-admin-probe.json',JSON.stringify({ownerFrames:colors.size,log},null,2));clearTimeout(hard);c.end();}
})();
