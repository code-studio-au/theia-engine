"""Independent demand/ownership oracle for bounded camera-world qualification."""
import json
from pathlib import Path
import struct
import subprocess
from qualify_world_retirement import resources


def require(value,message):
    if not value:raise RuntimeError(message)


class Oracle:
    def __init__(self,scene,renderer):
        self.scene=scene;self.renderer=str(Path(renderer).resolve());self.cache={}
        self.resources={i+offset:resources(e['cook']) for group,offset in [('quadrants',0),('fallbacks',4)]
            for i,e in enumerate(scene[group])}
        self.bounds=[]
        for entry in scene['quadrants']:
            blob=Path(entry['cook']['path']).read_bytes();meta,count=struct.unpack_from('<2I',blob,12)
            offset=76+((meta+3)&~3)
            points=list(struct.iter_unpack('<3f',blob[offset:offset+count*12]))
            self.bounds.append(([min(p[a] for p in points) for a in (0,1)],[max(p[a] for p in points) for a in (0,1)]))
        self.fine_gpu=[89376,89392,89184,89520]
        if len(scene['quadrants'][2]['cook']['textures'])==2:self.fine_gpu[2]=56112
    def camera(self,frame,period,width=960,height=540):
        key=(frame%period,period,width,height)
        if key not in self.cache:
            self.cache[key]=json.loads(subprocess.check_output([self.renderer,'--benchmark-reference','0',
                str(key[0]),str(period),str(width),str(height)],text=True,timeout=10))
        return self.cache[key]
    def plan(self,eye,fine,slots):
        eligible=[]
        for i,(lo,hi) in enumerate(self.bounds):
            closest=[min(hi[a],max(lo[a],eye[a])) for a in (0,1)]
            distance=sum((eye[a]-closest[a])**2 for a in (0,1))
            if distance<=(35 if i in fine else 20)**2:eligible.append((distance,i))
        eligible.sort();desired=[i for _,i in eligible[:slots]]
        return dict(desired=desired,loads=[i for i in desired if i not in fine],removals=sorted(fine-set(desired)),
            deferred=[i for _,i in eligible[slots:]])
    def charge(self,ids):return sum({k:v for i in ids for k,v in self.resources[i].items()}.values())
    def state(self,row,pending,caps):
        selected=row['selected'];require(len(selected)==4 and all(v in (i,i+4) for i,v in enumerate(selected)),'Bad region selection')
        fine={i for i in selected if i<4};ids=set(range(4,8))|fine
        future={pending['region']} if pending and pending['pending'] else set()
        encoded=self.charge(ids|future)
        images=sum(len(self.scene['quadrants' if i<4 else 'fallbacks'][i%4]['cook']['textures']) for i in ids)
        gpu=334368+sum(self.fine_gpu[i] for i in fine)
        require(row['protected_epochs']==[1]*4 and row['owned_chunks']==len(ids) and row['images']==images and
            row['image_bytes']==images*16384 and row['physical_bytes']==gpu and row['cpu_unique_bytes']==encoded,
            'CPU/current GPU ownership ledger differs')
        pending_gpu=sum(self.fine_gpu[i] for i in future)
        pending_images=sum(len(self.scene['quadrants'][i]['cook']['textures'])*16384 for i in future)
        if pending:require(pending['pending_gpu_bytes']==pending_gpu and pending['pending_image_bytes']==pending_images,'Pending GPU ledger differs')
        require(encoded<=caps[0] and gpu+pending_gpu<=caps[1] and images*16384+pending_images<=caps[2],'Current/pending cap exceeded')
        return fine
    def verify(self,report,frames,caps):
        require(report['completed'] and 0<len(report['history'])<=64,'Incomplete or unbounded camera history')
        history=report['history'];slots=report['fine_slots'];period=report['route_frames'];dropped=report['dropped_history']
        expected_events=1+sum(report[k] for k in ('requests','uploads','publications','evictions','cancellations','cpu_deferrals','gpu_deferrals'))
        require(len(history)+dropped==expected_events,'History ring accounting differs')
        counts={kind:0 for kind in ('initial','requested','pending','published','evicted','cancelled','deferred_cpu','deferred_gpu')}
        previous_epoch=None;previous_fine=None;previous_frame=-1;blocked={};pending_region=None
        for index,row in enumerate(history):
            kind=row['kind'];require(kind in counts,'Unknown event kind');counts[kind]+=1
            require(previous_frame<=row['frame']<frames,'Event frame ordering differs');previous_frame=row['frame']
            fine=self.state(row,row['pending'],caps);require(len(fine)<=slots,'Fine slot budget exceeded')
            if kind=='initial':
                require(not dropped and index==0 and row['epoch']==1 and not fine and row['plan'] is None,'Bad coarse bootstrap')
                previous_epoch=1;previous_fine=set();continue
            plan=row['plan'];candidate=plan.get('candidate')
            before=fine.copy()
            if kind=='published':
                require(candidate in fine,'Published request region missing');before.remove(candidate)
            elif kind=='evicted':
                require(plan['removals'],'Eviction without demand removal');before.add(plan['removals'][0])
            if previous_fine is not None:require(before==previous_fine,'Publication changes unrelated fine regions')
            reference=self.camera(row['frame'],period)
            require(plan['eye']==reference['eye'],'Camera input differs from benchmark route')
            expected=self.plan(plan['eye'],before,slots)
            require({k:plan[k] for k in expected}==expected,'Independent distance/retention plan differs')
            mutation=kind in ('published','evicted')
            if previous_epoch is not None:require(row['epoch']==previous_epoch+int(mutation),'CPU epoch differs')
            epoch_before=row['epoch']-int(mutation)
            if kind=='requested':
                require(candidate in expected['loads'],'Requested chunk is not missing/wanted')
                if not dropped or report['cpu_deferrals']+report['gpu_deferrals']==0 or previous_epoch!=history[0]['epoch']:
                    eligible=[i for i in expected['loads'] if blocked.get(i)!=epoch_before]
                    require(eligible and candidate==eligible[0],'Blocked/priority request selection differs')
                pending_region=candidate
            elif kind=='pending':
                require(row['pending'] and row['pending']['pending'] and row['pending']['region']==candidate,'Pending reservation absent')
                if pending_region is not None:require(candidate==pending_region,'Pending request changed region')
                pending_region=candidate
            elif kind=='published':
                require(candidate in expected['loads'] and not expected['removals'],'Published obsolete fine data')
                if pending_region is not None:require(candidate==pending_region,'Published wrong request')
                pending_region=None
            elif kind=='evicted':require(plan['removals'][0] not in fine,'Evicted fine chunk still selected')
            elif kind=='cancelled':
                cancelled=plan['cancelled'];require(cancelled['region'] not in expected['desired'] and not row['pending'],'Wanted request cancelled')
                pending_region=None
            elif kind.startswith('deferred_'):
                require(candidate in expected['loads'] and not row['pending'],'Deferred wrong request')
                if kind=='deferred_cpu':require(self.charge(set(range(4,8))|before|{candidate})>caps[0],'False CPU capacity deferral')
                else:
                    gpu=334368+sum(self.fine_gpu[i] for i in before|{candidate})
                    images=sum(len(self.scene['quadrants' if i<4 else 'fallbacks'][i%4]['cook']['textures'])*16384 for i in set(range(4,8))|before|{candidate})
                    require(gpu>caps[1] or images>caps[2],'False GPU/image capacity deferral')
                blocked[candidate]=epoch_before;pending_region=None
            previous_epoch=row['epoch'];previous_fine=fine
        final=report['final'];self.state(final,report['pending'],caps)
        require(final['frame']==frames and final['selected']==history[-1]['selected'] and final['epoch']==history[-1]['epoch'] and
            final['epoch']==1+report['publications']+report['evictions'],'Final publication state differs')
        if not dropped:
            for kind,key in [('requested','requests'),('pending','uploads'),('published','publications'),('evicted','evictions'),
                ('cancelled','cancellations'),('deferred_cpu','cpu_deferrals'),('deferred_gpu','gpu_deferrals')]:
                require(counts[kind]==report[key],'Event counter differs')
        require(report['peak_cpu_bytes']<=caps[0] and report['peak_gpu_bytes']<=caps[1] and report['peak_image_bytes']<=caps[2],
            'Observed complete/pending peak exceeds caps')
        return dict(events_checked=len(history),truncated_prefix=bool(dropped),epoch=final['epoch'])
