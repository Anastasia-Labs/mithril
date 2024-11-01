"use strict";(self.webpackChunkmithril_doc=self.webpackChunkmithril_doc||[]).push([[2868],{75671:(e,i,t)=>{t.r(i),t.d(i,{assets:()=>a,contentTitle:()=>o,default:()=>c,frontMatter:()=>s,metadata:()=>l,toc:()=>h});var n=t(74848),r=t(28453);const s={sidebar_position:3,sidebar_label:"Mithril signer"},o="Mithril signer node",l={id:"mithril/mithril-network/signer",title:"Mithril signer node",description:"The Mithril signer is a node that works transparently on top of the stake pool operators' Cardano nodes. It is responsible for independently signing the ledger state.",source:"@site/root/mithril/mithril-network/signer.md",sourceDirName:"mithril/mithril-network",slug:"/mithril/mithril-network/signer",permalink:"/doc/next/mithril/mithril-network/signer",draft:!1,unlisted:!1,editUrl:"https://github.com/input-output-hk/mithril/edit/main/docs/website/root/mithril/mithril-network/signer.md",tags:[],version:"current",sidebarPosition:3,frontMatter:{sidebar_position:3,sidebar_label:"Mithril signer"},sidebar:"mithrilSideBar",previous:{title:"Mithril aggregator",permalink:"/doc/next/mithril/mithril-network/aggregator"},next:{title:"Mithril client",permalink:"/doc/next/mithril/mithril-network/client"}},a={},h=[{value:"Individual signature production",id:"individual-signature-production",level:2},{value:"Interaction with the Mithril aggregator",id:"interaction-with-the-mithril-aggregator",level:2},{value:"Under the hood",id:"under-the-hood",level:2}];function d(e){const i={a:"a",admonition:"admonition",h1:"h1",h2:"h2",header:"header",img:"img",li:"li",p:"p",strong:"strong",ul:"ul",...(0,r.R)(),...e.components};return(0,n.jsxs)(n.Fragment,{children:[(0,n.jsx)(i.header,{children:(0,n.jsx)(i.h1,{id:"mithril-signer-node",children:"Mithril signer node"})}),"\n",(0,n.jsx)(i.admonition,{type:"info",children:(0,n.jsx)(i.p,{children:"The Mithril signer is a node that works transparently on top of the stake pool operators' Cardano nodes. It is responsible for independently signing the ledger state."})}),"\n",(0,n.jsx)(i.admonition,{type:"tip",children:(0,n.jsxs)(i.ul,{children:["\n",(0,n.jsxs)(i.li,{children:["\n",(0,n.jsxs)(i.p,{children:["For more information about the ",(0,n.jsx)(i.strong,{children:"Mithril protocol"}),", see the ",(0,n.jsx)(i.a,{href:"/doc/next/mithril/mithril-protocol/protocol",children:"protocol in depth"})," overview."]}),"\n"]}),"\n",(0,n.jsxs)(i.li,{children:["\n",(0,n.jsxs)(i.p,{children:["For more information about the ",(0,n.jsx)(i.strong,{children:"Mithril signer"}),", see the ",(0,n.jsx)(i.a,{href:"/doc/next/manual/developer-docs/nodes/mithril-signer",children:"developer manual"}),"."]}),"\n"]}),"\n"]})}),"\n",(0,n.jsx)(i.h2,{id:"individual-signature-production",children:"Individual signature production"}),"\n",(0,n.jsx)(i.p,{children:"The Mithril signer is a node representing a portion of the total stake within the Cardano network. This permits it to engage in Mithril multi-signature creation in proportion to its stake share. The principle is straightforward: a greater stake share corresponds to a more substantial contribution to the production of multi-signatures."}),"\n",(0,n.jsx)(i.p,{children:"To produce an individual signature, a Mithril signer must also be aware of all the other Mithril signers that can potentially contribute."}),"\n",(0,n.jsxs)(i.p,{children:["For the protocol to maintain its security, the Mithril signer must autonomously compute the messages (or digests) that require signing. To accomplish this, the signer heavily depends on the ",(0,n.jsx)(i.strong,{children:"consensus"})," mechanism of the Cardano network, which ensures that all network nodes locally store identical data (following a specific delay)."]}),"\n",(0,n.jsx)(i.p,{children:"If certain nodes are not fully synchronized or exhibit adversarial behavior, they will be unable to contribute, either:"}),"\n",(0,n.jsxs)(i.ul,{children:["\n",(0,n.jsx)(i.li,{children:"Because they do not sign the same message (as they use different data from what the rest of the network agrees upon)"}),"\n",(0,n.jsx)(i.li,{children:"Or they are not entitled to sign (because they are not true holders of the stake share they used to sign)."}),"\n"]}),"\n",(0,n.jsx)(i.h2,{id:"interaction-with-the-mithril-aggregator",children:"Interaction with the Mithril aggregator"}),"\n",(0,n.jsx)(i.p,{children:"In its initial version, the Mithril signer collaborates with other Mithril signers through a central Mithril aggregator that serves as a facilitator, avoiding the necessity for direct signer-to-signer interactions."}),"\n",(0,n.jsx)(i.p,{children:"Ultimately, any signer will have the potential to function as a Mithril aggregator, enhancing decentralization within the Mithril network."}),"\n",(0,n.jsx)(i.p,{children:"The Mithril signer establishes a connection with the Mithril aggregator for the following purposes:"}),"\n",(0,n.jsxs)(i.ul,{children:["\n",(0,n.jsx)(i.li,{children:"Obtaining the presently used protocol parameters"}),"\n",(0,n.jsx)(i.li,{children:"Registering its verification keys (public keys)"}),"\n",(0,n.jsx)(i.li,{children:"Receiving the verification keys of all other declared signers, available for the upcoming message signing"}),"\n",(0,n.jsx)(i.li,{children:"Sending the single signatures of locally computed messages (which will ideally be combined into multi-signatures by the aggregator)."}),"\n"]}),"\n",(0,n.jsx)(i.p,{children:"This process is summarized in the following diagram:"}),"\n",(0,n.jsx)("img",{src:t(14294).A,style:{background:"white"},alt:"signer workflow"}),"\n",(0,n.jsx)(i.h2,{id:"under-the-hood",children:"Under the hood"}),"\n",(0,n.jsxs)(i.p,{children:["In its initial version, the ",(0,n.jsx)(i.strong,{children:"Mithril signer"})," consists of a primary component:"]}),"\n",(0,n.jsxs)(i.ul,{children:["\n",(0,n.jsxs)(i.li,{children:["A runtime powered by a state machine:","\n",(0,n.jsxs)(i.ul,{children:["\n",(0,n.jsx)(i.li,{children:"The runtime operates synchronously and is programmed to run at consistent intervals"}),"\n",(0,n.jsxs)(i.li,{children:["Four potential states exist: ",(0,n.jsx)(i.strong,{children:"INIT"}),", ",(0,n.jsx)(i.strong,{children:"UNREGISTERED"}),", ",(0,n.jsx)(i.strong,{children:"READY TO SIGN"})," and ",(0,n.jsx)(i.strong,{children:"REGISTERED NOT ABLE TO SIGN"}),"."]}),"\n",(0,n.jsx)(i.li,{children:"The runtime effectively manages state transitions"}),"\n",(0,n.jsx)(i.li,{children:"The runtime's framework is depicted in the diagram below:"}),"\n"]}),"\n"]}),"\n"]}),"\n",(0,n.jsx)(i.p,{children:(0,n.jsx)(i.img,{alt:"Signer Runtime",src:t(74158).A+"",width:"614",height:"1628"})})]})}function c(e={}){const{wrapper:i}={...(0,r.R)(),...e.components};return i?(0,n.jsx)(i,{...e,children:(0,n.jsx)(d,{...e})}):d(e)}},14294:(e,i,t)=>{t.d(i,{A:()=>n});const n=t.p+"assets/images/signer-workflow-0099dc5e6cbaa76fca1cf084b510003e.png"},74158:(e,i,t)=>{t.d(i,{A:()=>n});const n=t.p+"assets/images/signer-runtime-a358e45c2dc6f03c89cb511f84b4af4f.jpg"},28453:(e,i,t)=>{t.d(i,{R:()=>o,x:()=>l});var n=t(96540);const r={},s=n.createContext(r);function o(e){const i=n.useContext(s);return n.useMemo((function(){return"function"==typeof e?e(i):{...i,...e}}),[i,e])}function l(e){let i;return i=e.disableParentContext?"function"==typeof e.components?e.components(r):e.components||r:o(e.components),n.createElement(s.Provider,{value:i},e.children)}}}]);