// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;

// LOCAL BACKEND INTEGRATION FIXTURE ONLY. Not audited and never a production deployment.
contract FixtureToken {
    mapping(address=>uint256) public balanceOf;
    mapping(address=>mapping(address=>uint256)) public allowance;
    bool public paused;
    function mint(address to,uint256 amount) external { balanceOf[to]+=amount; }
    function setPaused(bool value) external { paused=value; }
    function approve(address spender,uint256 amount) external returns(bool){allowance[msg.sender][spender]=amount;return true;}
    function transferFrom(address from,address to,uint256 amount) external returns(bool){require(!paused);require(allowance[from][msg.sender]>=amount);allowance[from][msg.sender]-=amount;return move(from,to,amount);}
    function transfer(address to,uint256 amount) external returns(bool){require(!paused);return move(msg.sender,to,amount);}
    function move(address from,address to,uint256 amount) private returns(bool){require(balanceOf[from]>=amount);balanceOf[from]-=amount;balanceOf[to]+=amount;return true;}
}
contract FixtureWallet {
    address public owner;
    bool public enabled=true;
    constructor(address value){owner=value;}
    function setEnabled(bool value) external {require(msg.sender==owner);enabled=value;}
    function isValidSignature(bytes32 h,bytes calldata sig) external view returns(bytes4){if(!enabled||sig.length!=65)return 0xffffffff;bytes32 r;bytes32 s;uint8 v;assembly{r:=calldataload(sig.offset) s:=calldataload(add(sig.offset,32)) v:=byte(0,calldataload(add(sig.offset,64)))}return ecrecover(h,v,r,s)==owner?bytes4(0x1626ba7e):bytes4(0xffffffff);}
}
