// Benchmark HUD only; reuse the existing repo-native font and number formatting.
uint worldHudMeshlets(bool total) {
    uint count=0;
    [unroll] for(uint i=0;i<4;++i)count+=total ? worldGeometry[i].Load(20):worldHudWork[i].Load(36);
    return count;
}
uint hudCharacter(uint line,uint column) {
    if(line==0) {uint title[14]={84,72,69,73,65,32,83,84,82,69,65,77,69,68};if(column<14)return title[column];return 32;}
    if(line==1) {uint prefix[4]={70,80,83,32};if(column<4)return prefix[column];return hudDecimal(sceneCamera.hudPerf.x,1,column-4);}
    if(line==2 || line==3) {uint prefix[7]={67,80,85,32,77,83,32};if(line==3)prefix[0]=71;if(column<7)return prefix[column];return hudDecimal(line==2 ? sceneCamera.hudPerf.y:sceneCamera.hudPerf.z,3,column-7);}
    if(line==4) {
        uint prefix[9]={77,69,83,72,76,69,84,83,32};if(column<9)return prefix[column];uint col=column-9;
        uint count=worldHudMeshlets(false),total=worldHudMeshlets(true),n=hudDigits(count),m=hudDigits(total);
        if(col<n)return hudUInt(count,col);if(col==n)return 47;
        if(col<n+1+m)return hudUInt(total,col-n-1);
        uint suffix[5]={32,84,69,88,32};if(col<n+1+m+5)return suffix[col-n-1-m];
        return hudUInt(sceneCamera.hudCounts.y,col-n-1-m-5);
    }
    if(line==5 || line==6) {
        uint asset[10]={65,83,83,69,84,32,77,73,66,32};uint draw[9]={68,82,65,87,32,77,73,66,32};uint length=line==5 ? 10:9;
        if(column<length) {if(line==5)return asset[column];return draw[column];}
        return hudDecimal(line==5 ? sceneCamera.hudCounts.x:sceneCamera.hudScene.z,line==5 ? 3:1,column-length);
    }
    if(line==7) {
        uint prefix[4]={80,85,66,32};if(column<4)return prefix[column];uint col=column-4,n=hudDigits(sceneCamera.hudCounts.z);
        if(col<n)return hudUInt(sceneCamera.hudCounts.z,col);
        uint suffix[7]={32,69,86,73,67,84,32};if(col<n+7)return suffix[col-n];return hudUInt(sceneCamera.hudCounts.w,col-n-7);
    }
    if(line==8) {
        uint prefix[6]={80,72,65,83,69,32};if(column<6)return prefix[column];uint col=column-6;
        if(col==0)return hudUInt(sceneCamera.hudScene.x,0);
        uint time[3]={32,84,32};if(col<4)return time[col-1];
        uint digits=hudDigits(sceneCamera.hudScene.y/10)+2;
        if(col<4+digits)return hudDecimal(sceneCamera.hudScene.y,1,col-4);
        uint suffix[5]={32,68,69,70,32};if(col<4+digits+5)return suffix[col-4-digits];
        return hudUInt(sceneCamera.hudScene.w,col-4-digits-5);
    }
    if(line==9) {
        uint prefix[8]={80,82,79,88,73,69,83,32};if(column<8)return prefix[column];uint col=column-8,count=mixedProxyWork.Load(0),n=hudDigits(count);
        if(col<n)return hudUInt(count,col);if(col==n)return 47;return hudUInt(mixedProxyWork.Load(16),col-n-1);
    }
    if(line==10) {
        uint prefix[5]={78,69,65,82,32};if(column<5)return prefix[column];uint col=column-5,count=mixedProxyWork.Load(20),n=hudDigits(count);
        if(col<n)return hudUInt(count,col);uint suffix[5]={32,70,65,82,32};if(col<n+5)return suffix[col-n];return hudUInt(mixedProxyWork.Load(24),col-n-5);
    }
    return 32;
}
