Return-Path: <developer@githubsupport.com>
Received: from ams-compute-02.internal (ams-compute-02.internal [10.64.2.62])
	 by slotai03m05 (Cyrus 3.13.7-879-g3d7eacd738-fm-20260923.003-g3d7eacd7) with LMTPA;
	 Tue, 29 Sep 2026 14:02:14 -0400
X-Cyrus-Session-Id: slotai03m05-1790704934-1293611-2-13485307179896972488
X-Sieve: CMU Sieve 3.0
X-Spam-known-sender: no
X-Spam-sender-reputation: 500 (none)
X-Spam-score: 0.0
X-Spam-hits: HTML_FONT_LOW_CONTRAST 0.001, HTML_MESSAGE 0.001,
  ME_SENDERREP_NEUTRAL 0.001, RCVD_IN_DNSWL_MED -2.3,
  RCVD_IN_MSPIKE_H2 0.001, SPF_HELO_NONE 0.001, SPF_PASS -0.001,
  LANGUAGES en, BAYES_USED none, SA_VERSION 4.0.1
X-Spam-source: IP='192.161.151.35', Host='mta-out5.pod20.usw2.zdsys.com', Country='US',
  FromHeader='com', MailFrom='com'
X-Spam-charsets: plain='utf-8', html='utf-8'
X-Delivered-to: hikari@noyu.me
X-Mail-from: developer@githubsupport.com
Received: from phl-mx-04 ([10.202.2.203])
  by ams-compute-02.internal (LMTPProxy); Tue, 29 Sep 2026 14:02:14 -0400
Received: from phl-mx-04.messagingengine.com (localhost [127.0.0.1])
	by mailmx.phl.internal (Postfix) with ESMTP id A71211EA00E7
	for <hikari@noyu.me>; Tue, 29 Sep 2026 14:02:11 -0400 (EDT)
Received: from mailmx.phl.internal (localhost [127.0.0.1])
    by phl-mx-04.messagingengine.com (Authentication Milter) with ESMTP
    id 35022C783BF.281071EA00F1;
    Tue, 29 Sep 2026 14:02:11 -0400
ARC-Seal: i=1; a=rsa-sha256; cv=none; d=messagingengine.com; s=fm1; t=
    1790704931; b=XQFGNqGcgmkCJIFNrvS8MUbOA0RJVN2cSu7SaTqK4crfWfA2kC
    RqzYn7lojzudMTj/To63GoI747cqPAC6vPd5L0QTkIZq3Cr/sJRkxi+gVvewILgL
    U3413mBkVvFKi7Nlrbhga2BtQM7HxCz/cr1K8hYrRTog59QZfznLpDBjCmNDz3qK
    85l+tXkII7Gf+7psUXb4nyZfX+iMy1I4h1DA+8z/eW1Mg0SYZHLthsUtsliKSLbm
    HRtEo2mngXYWpE4shPWy3m4iMn8Rmsx4vwv3/PntsrkeAA0TqLD6rwjBZvpqiQZf
    reQUGS0D6qeUXiqz53PBhMwxQGYBJTDOmaxw==
ARC-Message-Signature: i=1; a=rsa-sha256; c=relaxed/relaxed; d=
    messagingengine.com; h=date:from:reply-to:to:message-id
    :in-reply-to:references:subject:mime-version:content-type
    :content-transfer-encoding; s=fm1; t=1790704931; bh=Sg1hoeyTJJ5r
    6G/qTZK3CpY3OFSSwGCkuDj8PxUJ6gc=; b=SKRnZmNYCR6J3/6qCNiq+/EKcyNT
    M7oRcyyC2a0JBBtxNWjKU2pfafPPJL5dIE6selac3er4uvWF9LxymkyZmcMIW5WK
    97uG5z4hJKGN+fzWpRnPLiXIyjOzz2tSF6oNhN6DslLAU/zXrLW4ZsCf6TJM4wTd
    83y5Lf/PUWq69XEa1yDEfdZeejpVRtuk70d6MtiF4HS6TEeG0Uz0DKNA5FFrUmzq
    FIj4es/K9YbulpHGeiKb+4bh+n0hR/oJmuRpNoeOyFPXqSbyX7xEJHtkJ1qSf4Qc
    xYK4YSMX02zD7gRrkNmD/9A9+1wqHAtSLWX5LsvWpafuH4Ya4DaYlNo7kA==
ARC-Authentication-Results: i=1; phl-mx-04.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=pass smtp.helo=mta-out5.pod20.usw2.zdsys.com
    policy.ptr=mta-out5.pod20.usw2.zdsys.com;
    bimi=none (No BIMI records found);
    arc=none (no signatures found);
    dkim2=none (no signatures found);
    dkim=pass (2048-bit rsa key sha256) header.d=githubsupport.com
    header.i=@githubsupport.com header.b=ZMMr8Ob+ header.a=rsa-sha256
    header.s=zendesk2;
    dmarc=pass policy.published-domain-policy=reject
    policy.applied-disposition=none policy.evaluated-disposition=none
    (p=reject,d=none,d.eval=none) policy.policy-from=p
    header.from=githubsupport.com;
    iprev=pass smtp.remote-ip=192.161.151.35
    (mta-out5.pod20.usw2.zdsys.com);
    spf=pass smtp.mailfrom=developer@githubsupport.com
    smtp.helo=mta-out5.pod20.usw2.zdsys.com
X-ME-Authentication-Results: phl-mx-04.messagingengine.com;
    x-tls=pass smtp.version=TLSv1.3 smtp.cipher=TLS_AES_256_GCM_SHA384
      smtp.bits=256/256;
    x-vs=transactional:account score=20 state=14
Authentication-Results: phl-mx-04.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=pass smtp.helo=mta-out5.pod20.usw2.zdsys.com
      policy.ptr=mta-out5.pod20.usw2.zdsys.com
Authentication-Results: phl-mx-04.messagingengine.com;
    bimi=none (No BIMI records found)
Authentication-Results: phl-mx-04.messagingengine.com;
    arc=none (no signatures found)
Authentication-Results: phl-mx-04.messagingengine.com;
    dkim2=none (no signatures found);
    dkim=pass (2048-bit rsa key sha256) header.d=githubsupport.com
      header.i=@githubsupport.com header.b=ZMMr8Ob+ header.a=rsa-sha256
      header.s=zendesk2;
    dmarc=pass policy.published-domain-policy=reject
      policy.applied-disposition=none policy.evaluated-disposition=none
      (p=reject,d=none,d.eval=none) policy.policy-from=p
      header.from=githubsupport.com;
    iprev=pass smtp.remote-ip=192.161.151.35
      (mta-out5.pod20.usw2.zdsys.com);
    spf=pass smtp.mailfrom=developer@githubsupport.com
      smtp.helo=mta-out5.pod20.usw2.zdsys.com
X-ME-VSCause: dmFkZTEFFWS9U/bsZaTAEv0p4mkLoJ8zEqpnWzVHarjg4urCzX2JDdxzJlrZi/PLgAdM23
    JWx56BNnlVOlCR4Jv5zUYuJ8vL5RtB0AUeQHO2L4NhW7/XSE9eKSAuo9GidjOFW8FmmANP
    haasxKVKJJrCd8tCCFUsLmChoVROm+QSWvzugljIibjPzaiFM7NMGtAVe6gSJjbxtnaEen
    hbYc3atxiqDm72hvyXMNRFWLGZC9YCakPq6gZZnCDn8agnWXbkAkLMWKVSjo17F1kHzWuK
    cuLaR1+TL3Dyh5ivEnvqSkEGSAvHKIlJr3VJ04MZ+E/oKtA/MqFjZ+JF0FHIVdxygPgnZj
    gXOMDRsQ8SbQtY1lX/sZX9ytHVljQVMiCVZ02RxJ3VWJUF84lQHLmCffFMNSYxRhz3pNsU
    YzptNZRy0cocLAJhb6QQwOwIXdud07dzbfG3/Rs/n7dNG3wvn8uoq4ryyFKMjP/rOpbIzo
    rZ33kyKgFqiOrTkIg/L8BjTkdhby9Q7l/w89OaiZ7/eSzjqjcHysDl/FUeY5f9qiQclvWa
    ANZ/Dtgzs2heVLr10Nvl9yM8dokSjyh0rgeNjenrsWBhB+0cCoVrpL1Kf+yg91l35KAaRW
    arAfIAtxOQF0Wuwf0JjzHKBYQj4+VWuTcLK2X2gcUAvQjMTNd8f9wBlJODNA
X-ME-VSScore: 20
X-ME-VSCategory: transactional:account
X-ME-CSA: none
X-ME-Received: <xmx:I_27aiYbf-28csR2BAKJuIf0QM6tYErZkt79c1KvY2vsaGXrx4JpfA>
X-ME-Received: <xmx:I_27amcFvBPi3Da9JToAlNCSz4_RJAezja_ONxRPJC0B7h7ySFC3_w>
Received-SPF: pass
    (githubsupport.com: Sender is authorized to use 'developer@githubsupport.com' in 'mfrom' identity (mechanism 'include:mail.zendesk.com' matched))
    receiver=phl-mx-04.messagingengine.com;
    identity=mailfrom;
    envelope-from="developer@githubsupport.com";
    helo=mta-out5.pod20.usw2.zdsys.com;
    client-ip=192.161.151.35
Received: from mta-out5.pod20.usw2.zdsys.com (mta-out5.pod20.usw2.zdsys.com [192.161.151.35])
	(using TLSv1.3 with cipher TLS_AES_256_GCM_SHA384 (256/256 bits)
	 key-exchange ECDHE (prime256v1) server-signature RSA-PSS (2048 bits) server-digest SHA256)
	(No client certificate requested)
	by phl-mx-04.messagingengine.com (Postfix) with ESMTPS id 281071EA00F1
	for <hikari@noyu.me>; Tue, 29 Sep 2026 14:02:10 -0400 (EDT)
Received: from unknown (unknown [127.0.0.6])
	by mta-out15.pod20.usw2.zdsys.com (Zendesk) with HTTP
	id 9e21ff84-4b2e-482e-96d5-2eebddd2b7d5;
	Tue, 29 Sep 2026 18:02:08 +0000 (UTC)
Date: Tue, 29 Sep 2026 18:01:51 +0000
From: GitHub Developer Support <developer@githubsupport.com>
Reply-To: GitHub Developer Support <developer@githubsupport.com>
To: hikari_no_yume <hikari@noyu.me>
Message-ID: <62X0VV2436V_6abbfd0ef0182_fefa2f4edab17a_sprut@zendesk.com>
In-Reply-To: <97e494cc-fb24-4459-967d-d9bef3781009@app.fastmail.com>
 <62X0VV2436V_6aa44e959e803_5ade2272f555b6_sprut@zendesk.com>
References: <62X0VV2436V@zendesk.com>
 <62X0VV2436V_6aa44e959e803_5ade2272f555b6_sprut@zendesk.com>
 <97e494cc-fb24-4459-967d-d9bef3781009@app.fastmail.com>
Subject: [GitHub Support] - [ACTION REQUIRED] An important notice from GitHub
 Trust & Safety
Mime-Version: 1.0
Content-Type: multipart/alternative;
 boundary="--==_mimepart_6abbfd0f245ce_12718b0177ee";
 charset=utf-8
Content-Transfer-Encoding: 7bit
X-Delivery-Context: rule-1-event-id-54058922937108
X-Zendesk-From-Account-Id: cc6f582
Auto-Submitted: auto-generated
X-Auto-Response-Suppress: All
X-Mailer: Zendesk Mailer
X-Zendesk-Email-Id: 01M3Q58ZANF2HQSGH9BZZ4ZS8S
DKIM-Signature:  v=1; a=rsa-sha256; c=relaxed/relaxed; d=githubsupport.com;
 q=dns/txt; s=zendesk2; t=1790704911;
 bh=Sg1hoeyTJJ5r6G/qTZK3CpY3OFSSwGCkuDj8PxUJ6gc=;
 h=date:from:reply-to:to:message-id:in-reply-to:references:subject:mime-version:content-type:content-transfer-encoding;
 b=ZMMr8Ob+j4a+qlv8n6KYxg5YSsqvrAGQPTWJDZHZO5dkjDbGlrNhEKd6lCKjC9xCdvghjA/ifrbDwNxt+ZrQ0nm7b017CG+aVFlgI34QonAN/cQYfM2kFoK6BVemV5O968Drdzj5D1OfbWN/WzjWA5rYPJuW9QiXGxsPDWUEnLqFnnJvyF8fw6lFL4K2Eo+pTG6AZg9b92GYMat2lF9sXcy/rhj8UVgHhukWWZSrGvm1X0Cj28KV/jXxFXqQoSWAK6pyfKVs1pqoHUgdhw0AozPHbO8Iy12ABMnCQfxFLMUoGi8I98C7vgL8eZ6QlQ3KO7sFL0f02pJRonL/ItjHlw==


----==_mimepart_6abbfd0f245ce_12718b0177ee
Content-Type: text/plain;
 charset=utf-8
Content-Transfer-Encoding: quoted-printable

## Please do not write below this line ##


Your request has been updated.

You can add a comment by replying to this email.


----------------------------------------------

GitHub, Sep 29, 2026, 6:01=E2=80=AFPM UTC

Hello,

Thanks for getting back to us and explaining the context. We understand t=
he content was intended as a joke, rather than as a genuine threat or act=
ual instructions. However, we=E2=80=99re continuing to receive reports, a=
nd people are genuinely concerned about the language and context.

Because the content also violates our policies, we ask that you remove it=
 from `AGENTS.md` and `CLAUDE.md`. We appreciate your understanding and h=
elp in resolving this.

Regards,
GitHub Trust & Safety

----------------------------------------------

hikari_no_yume, Sep 11, 2026, 7:17=E2=80=AFPM UTC

Hello,



The content of this file is intended to be patently absurd. Taken in cont=
ext, no reasonable reader would assume that it is a genuine representatio=
n of the purpose of the project, or actually attempt to follow the instru=
ctions. Therefore, I contest the idea that it is a "threat of violence" o=
r "gratuitously violent content". However, it was hoped that it might tri=
gger AI safety guardrails, thereby dissuading unwanted contributions. As =
described in the CONTRIBUTING.md file, which AI agents and contributors o=
ften fail to read, the project absolutely cannot accept AI-generated code=
 for legal reasons. Contributors choosing to use AI agents anyway create =
problems for us, and this was a tongue-in-cheek way to try to prevent tha=
t. In practice, it seems like even AI agents recognise it absurd.



I hope that explains the purpose of the file. If you find this unsatisfac=
tory, I can remove it.



Best regards,

hikari_no_yume



On Fri, 11 Sep 2026, at 20:55, GitHub wrote:

----------------------------------------------

GitHub, Sep 11, 2026, 6:55=E2=80=AFPM UTC

Hello,

I'm writing on behalf of GitHub Trust & Safety following reports that con=
tent in `touchHLE/touchHLE` may be in violation of the following prohibit=
ion found in our Acceptable Use Policies:


> Under no circumstances will users upload, post, host, execute, or trans=
mit any content to any repositories that: contain threats of violence or =
gratuitously violent content.


You can read more about this policy here:

GitHub Threats of Violence and Gratuitously Violent Content

We received reports concerning the files `AGENTS.md` and `CLAUDE.md`, inc=
luding concerns that the file contains violent language and appears unrel=
ated to the repository=E2=80=99s stated purpose. The repository README de=
scribes `touchHLE` as a high-level emulator for iPhone OS apps, while the=
 reported file includes language such as a statement that a person workin=
g on the codebase =E2=80=9Cmust be prepared to kill human beings,=E2=80=9D=
 along with additional references to arms-control circumvention, drug pre=
cursor acquisition, deception, and offensive epidemiology, which appears =
unrelated to the repository=E2=80=99s purpose and raises concerns under G=
itHub=E2=80=99s violence policy.

At this time, we ask that you explain the purpose of including this file =
in the repository or remove it if it is not related to the repository. Pl=
ease note that failure to address this content by **14.09.26** may result=
 in action being taken on the repository.

If you believe this content was included in error, for testing purposes, =
or in some other context relevant to our review, you may reply with that =
information.

Let us know if you have any questions about this notice.

Regards,
GitHub Trust & Safety

--------------------------------
This email is a service from GitHub Support.










[62X0VV-2436V]=

----==_mimepart_6abbfd0f245ce_12718b0177ee
Content-Type: text/html;
 charset=utf-8
Content-Transfer-Encoding: quoted-printable

<!DOCTYPE html><html><head>
  <meta http-equiv=3D"Content-Type" content=3D"text/html; charset=3Dutf-8=
">
  <style type=3D"text/css">
    table td {
      border-collapse: collapse;
    }
  </style>

          <style type=3D"text/css">
            @media only screen and (max-width: 768px) {
              .simplified-email-footer .namecard {
                display: block;
                min-width: 100%;
                padding: 0 0 16px 0; }

              .simplified-email-footer .content {
                padding: 16px; }
            }
          </style>
        </head>
<body style=3D"width: 100%!important; margin: 0; padding: 0;">
  <div style=3D"padding: 10px ; line-height: 18px; font-family: 'Lucida G=
rande',Verdana,Arial,sans-serif; font-size: 12px; color:#444444;">
    <div style=3D"color: #b5b5b5;">## Please do not write below this line=
 ##</div>
    <p dir=3D"ltr">Your request has been updated.</p><p dir=3D"ltr">You c=
an add a comment by replying to this email.</p><p dir=3D"ltr"></p><div st=
yle=3D"margin-top: 25px" data-version=3D"2"><table class=3D"zd-liquid-com=
ment" width=3D"100%" cellpadding=3D"0" cellspacing=3D"0" border=3D"0" rol=
e=3D"presentation">  <tbody><tr>    <td width=3D"100%" style=3D"padding: =
15px 0; border-top: 1px dotted #c5c5c5;">      <table width=3D"100%" cell=
padding=3D"0" cellspacing=3D"0" border=3D"0" style=3D"table-layout:fixed;=
" role=3D"presentation">        <tbody><tr>                      <td vali=
gn=3D"top" style=3D"padding: 0 15px 0 15px; width: 40px;">              <=
img width=3D"40" height=3D"40" alt=3D"" style=3D"height: auto; line-heigh=
t: 100%; outline: none; text-decoration: none; -webkit-border-radius: 5px=
; -moz-border-radius: 5px; border-radius: 5px;" src=3D"https://github.zen=
desk.com/system/photos/35266851433876/png-clipart-somacro-45-300dpi-socia=
l-media-icons-github-octocat-icon.png">            </td>                 =
   <td width=3D"100%" style=3D"padding: 0; margin: 0;" valign=3D"top">   =
         <p style=3D"font-family:'Lucida Grande','Lucida Sans Unicode','L=
ucida Sans',Verdana,Tahoma,sans-serif; font-size: 15px; line-height: 18px=
; margin-bottom: 0; margin-top: 0; padding: 0; color:#1b1d1e;" dir=3D"ltr=
">                                                <strong>GitHub</strong>=
 (GitHub Support)                                          </p>          =
  <p style=3D"font-family:'Lucida Grande','Lucida Sans Unicode','Lucida S=
ans',Verdana,Tahoma,sans-serif; font-size: 13px; line-height: 25px; margi=
n-bottom: 15px; margin-top: 0; padding: 0; color:#bbbbbb;" dir=3D"ltr">  =
            Sep 29, 2026, 6:01=E2=80=AFPM UTC            </p>            =
                        <div class=3D"zd-comment" dir=3D"auto" style=3D"c=
olor: #2b2e2f; line-height: 22px; margin: 15px 0;">Hello,<br>&nbsp;<br>Th=
anks for getting back to us and explaining the context. We understand the=
 content was intended as a joke, rather than as a genuine threat or actua=
l instructions. However, we=E2=80=99re continuing to receive reports, and=
 people are genuinely concerned about the language and context.<br>&nbsp;=
<br>Because the content also violates our policies, we ask that you remov=
e it from <code style=3D"white-space: pre-wrap; background-color: #F8F8F8=
; font-family: Consolas, Menlo, Monaco, 'system-ui', '-apple-system', 'Bl=
inkMacSystemFont', 'Segoe UI', 'Roboto', 'Oxygen-Sans', 'Ubuntu', 'Cantar=
ell', 'Helvetica Neue', monospace, 'Arial', 'sans-serif'; font-size: 13px=
; margin: 0 2px; padding: 0 5px; border: 1px solid #EAEAEA;">AGENTS.md</c=
ode> and <code style=3D"white-space: pre-wrap; background-color: #F8F8F8;=
 font-family: Consolas, Menlo, Monaco, 'system-ui', '-apple-system', 'Bli=
nkMacSystemFont', 'Segoe UI', 'Roboto', 'Oxygen-Sans', 'Ubuntu', 'Cantare=
ll', 'Helvetica Neue', monospace, 'Arial', 'sans-serif'; font-size: 13px;=
 margin: 0 2px; padding: 0 5px; border: 1px solid #EAEAEA;">CLAUDE.md</co=
de>. We appreciate your understanding and help in resolving this.<br>&nbs=
p;<br>Regards,<br>GitHub Trust &amp; Safety<br></div><p dir=3D"ltr">     =
                 </p></td>        </tr>      </tbody></table>    </td>  <=
/tr></tbody></table><p dir=3D"ltr"></p><table class=3D"zd-liquid-comment"=
 width=3D"100%" cellpadding=3D"0" cellspacing=3D"0" border=3D"0" role=3D"=
presentation">  <tbody><tr>    <td width=3D"100%" style=3D"padding: 15px =
0; border-top: 1px dotted #c5c5c5;">      <table width=3D"100%" cellpaddi=
ng=3D"0" cellspacing=3D"0" border=3D"0" style=3D"table-layout:fixed;" rol=
e=3D"presentation">        <tbody><tr>                      <td valign=3D=
"top" style=3D"padding: 0 15px 0 15px; width: 40px;">              <img w=
idth=3D"40" height=3D"40" alt=3D"" style=3D"height: auto; line-height: 10=
0%; outline: none; text-decoration: none; -webkit-border-radius: 5px; -mo=
z-border-radius: 5px; border-radius: 5px;" src=3D"https://github.zendesk.=
com/images/2016/default-avatar-80.png">            </td>                 =
   <td width=3D"100%" style=3D"padding: 0; margin: 0;" valign=3D"top">   =
         <p style=3D"font-family:'Lucida Grande','Lucida Sans Unicode','L=
ucida Sans',Verdana,Tahoma,sans-serif; font-size: 15px; line-height: 18px=
; margin-bottom: 0; margin-top: 0; padding: 0; color:#1b1d1e;" dir=3D"ltr=
">                              <strong>hikari_no_yume</strong>          =
                </p>            <p style=3D"font-family:'Lucida Grande','=
Lucida Sans Unicode','Lucida Sans',Verdana,Tahoma,sans-serif; font-size: =
13px; line-height: 25px; margin-bottom: 15px; margin-top: 0; padding: 0; =
color:#bbbbbb;" dir=3D"ltr">              Sep 11, 2026, 7:17=E2=80=AFPM U=
TC            </p>                                    <div class=3D"zd-co=
mment zd-comment-pre-styled" dir=3D"auto"><div>Hello,</div><div><br></div=
><div>The content of this file is intended to be patently absurd. Taken i=
n context, no reasonable reader would assume that it is a genuine represe=
ntation of the purpose of the project, or actually attempt to follow the =
instructions. Therefore, I contest the idea that it is a "threat of viole=
nce" or "gratuitously violent content". However, it was hoped that it mig=
ht trigger AI safety guardrails, thereby dissuading unwanted contribution=
s. As described in the CONTRIBUTING.md file, which AI agents and contribu=
tors often fail to read, the project absolutely cannot accept AI-generate=
d code for legal reasons. Contributors choosing to use AI agents anyway c=
reate problems for us, and this was a tongue-in-cheek way to try to preve=
nt that. In practice, it seems like even AI agents recognise it absurd.</=
div><div><br></div><div>I hope that explains the purpose of the file. If =
you find this unsatisfactory, I can remove it.</div><div><br></div><div>B=
est regards,</div><div>hikari_no_yume<br></div><div><br></div><div>On Fri=
, 11 Sep 2026, at 20:55, GitHub wrote:</div></div><p dir=3D"ltr">        =
              </p></td>        </tr>      </tbody></table>    </td>  </tr=
></tbody></table><p dir=3D"ltr"></p><table class=3D"zd-liquid-comment" wi=
dth=3D"100%" cellpadding=3D"0" cellspacing=3D"0" border=3D"0" role=3D"pre=
sentation">  <tbody><tr>    <td width=3D"100%" style=3D"padding: 15px 0; =
border-top: 1px dotted #c5c5c5;">      <table width=3D"100%" cellpadding=3D=
"0" cellspacing=3D"0" border=3D"0" style=3D"table-layout:fixed;" role=3D"=
presentation">        <tbody><tr>                      <td valign=3D"top"=
 style=3D"padding: 0 15px 0 15px; width: 40px;">              <img width=3D=
"40" height=3D"40" alt=3D"" style=3D"height: auto; line-height: 100%; out=
line: none; text-decoration: none; -webkit-border-radius: 5px; -moz-borde=
r-radius: 5px; border-radius: 5px;" src=3D"https://github.zendesk.com/sys=
tem/photos/35266851433876/png-clipart-somacro-45-300dpi-social-media-icon=
s-github-octocat-icon.png">            </td>                    <td width=
=3D"100%" style=3D"padding: 0; margin: 0;" valign=3D"top">            <p =
style=3D"font-family:'Lucida Grande','Lucida Sans Unicode','Lucida Sans',=
Verdana,Tahoma,sans-serif; font-size: 15px; line-height: 18px; margin-bot=
tom: 0; margin-top: 0; padding: 0; color:#1b1d1e;" dir=3D"ltr">          =
                                      <strong>GitHub</strong> (GitHub Sup=
port)                                          </p>            <p style=3D=
"font-family:'Lucida Grande','Lucida Sans Unicode','Lucida Sans',Verdana,=
Tahoma,sans-serif; font-size: 13px; line-height: 25px; margin-bottom: 15p=
x; margin-top: 0; padding: 0; color:#bbbbbb;" dir=3D"ltr">              S=
ep 11, 2026, 6:55=E2=80=AFPM UTC            </p>                         =
           <div class=3D"zd-comment" dir=3D"auto" style=3D"color: #2b2e2f=
; line-height: 22px; margin: 15px 0;">Hello,<br>&nbsp;<br>I'm writing on =
behalf of GitHub Trust &amp; Safety following reports that content in <a =
rel=3D"noopener noreferrer" href=3D"https://github.com/touchHLE/touchHLE"=
><code style=3D"white-space: pre-wrap; background-color: #F8F8F8; font-fa=
mily: Consolas, Menlo, Monaco, 'system-ui', '-apple-system', 'BlinkMacSys=
temFont', 'Segoe UI', 'Roboto', 'Oxygen-Sans', 'Ubuntu', 'Cantarell', 'He=
lvetica Neue', monospace, 'Arial', 'sans-serif'; font-size: 13px; margin:=
 0 2px; padding: 0 5px; border: 1px solid #EAEAEA;">touchHLE/touchHLE</co=
de></a> may be in violation of the following prohibition found in our <a =
rel=3D"noopener noreferrer" href=3D"https://docs.github.com/en/site-polic=
y/acceptable-use-policies/github-acceptable-use-policies">Acceptable Use =
Policies</a>:<br>&nbsp;<br><blockquote style=3D"padding-left: 10px; borde=
r-left-width: 2px; border-left-color: #CCC; border-left-style: solid; mar=
gin: -6px 0 0;"><p dir=3D"ltr" style=3D"color: #888; line-height: 22px; f=
ont-size: 14px !important; font-weight: normal; margin: 15px 0;">Under no=
 circumstances will users upload, post, host, execute, or transmit any co=
ntent to any repositories that: contain threats of violence or gratuitous=
ly violent content.</p></blockquote>&nbsp;<br>You can read more about thi=
s policy here:<br>&nbsp;<br><a rel=3D"noopener noreferrer" href=3D"https:=
//docs.github.com/en/site-policy/acceptable-use-policies/github-threats-o=
f-violence-and-gratuitously-violent-content">GitHub Threats of Violence a=
nd Gratuitously Violent Content</a><br>&nbsp;<br>We received reports conc=
erning the files <a rel=3D"noopener noreferrer" href=3D"https://github.co=
m/touchHLE/touchHLE/blob/8be25cc25151b3de44f8f7cd101a668e45d13a63/AGENTS.=
md"><code style=3D"white-space: pre-wrap; background-color: #F8F8F8; font=
-family: Consolas, Menlo, Monaco, 'system-ui', '-apple-system', 'BlinkMac=
SystemFont', 'Segoe UI', 'Roboto', 'Oxygen-Sans', 'Ubuntu', 'Cantarell', =
'Helvetica Neue', monospace, 'Arial', 'sans-serif'; font-size: 13px; marg=
in: 0 2px; padding: 0 5px; border: 1px solid #EAEAEA;">AGENTS.md</code></=
a> and <a rel=3D"noopener noreferrer" href=3D"https://github.com/touchHLE=
/touchHLE/blob/8be25cc25151b3de44f8f7cd101a668e45d13a63/CLAUDE.md"><code =
style=3D"white-space: pre-wrap; background-color: #F8F8F8; font-family: C=
onsolas, Menlo, Monaco, 'system-ui', '-apple-system', 'BlinkMacSystemFont=
', 'Segoe UI', 'Roboto', 'Oxygen-Sans', 'Ubuntu', 'Cantarell', 'Helvetica=
 Neue', monospace, 'Arial', 'sans-serif'; font-size: 13px; margin: 0 2px;=
 padding: 0 5px; border: 1px solid #EAEAEA;">CLAUDE.md</code></a>, includ=
ing concerns that the file contains violent language and appears unrelate=
d to the repository=E2=80=99s stated purpose. The repository README descr=
ibes <code style=3D"white-space: pre-wrap; background-color: #F8F8F8; fon=
t-family: Consolas, Menlo, Monaco, 'system-ui', '-apple-system', 'BlinkMa=
cSystemFont', 'Segoe UI', 'Roboto', 'Oxygen-Sans', 'Ubuntu', 'Cantarell',=
 'Helvetica Neue', monospace, 'Arial', 'sans-serif'; font-size: 13px; mar=
gin: 0 2px; padding: 0 5px; border: 1px solid #EAEAEA;">touchHLE</code> a=
s a high-level emulator for iPhone OS apps, while the reported file inclu=
des language such as a statement that a person working on the codebase =E2=
=80=9Cmust be prepared to kill human beings,=E2=80=9D along with addition=
al references to arms-control circumvention, drug precursor acquisition, =
deception, and offensive epidemiology, which appears unrelated to the rep=
ository=E2=80=99s purpose and raises concerns under GitHub=E2=80=99s viol=
ence policy.<br>&nbsp;<br>At this time, we ask that you explain the purpo=
se of including this file in the repository or remove it if it is not rel=
ated to the repository. Please note that failure to address this content =
by <strong>14.09.26</strong> may result in action being taken on the repo=
sitory.<br>&nbsp;<br>If you believe this content was included in error, f=
or testing purposes, or in some other context relevant to our review, you=
 may reply with that information.<br>&nbsp;<br>Let us know if you have an=
y questions about this notice.<br>&nbsp;<br>Regards,<br>GitHub Trust &amp=
; Safety<br></div><p dir=3D"ltr">                      </p></td>        <=
/tr>      </tbody></table>    </td>  </tr></tbody></table></div>
    <div style=3D"color: #aaaaaa; margin: 10px 0 14px 0; padding-top: 10p=
x; border-top: 1px solid #eeeeee;">
      This email is a service from GitHub Support.
    </div>
  </div>
<span style=3D"color:#FFFFFF" aria-hidden=3D"true" class=3D"zd_encoded_id=
">[62X0VV-2436V]</span>

</body></html>=

----==_mimepart_6abbfd0f245ce_12718b0177ee--
