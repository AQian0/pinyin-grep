// Mixed-language sample used by tests/integration.rs.
// English identifiers should be ignored; Pinyin ones should be detected.

const xinXi = { kind: "info" };
const userName = "ada";

function huoQuYongHu() {
  return xinXi;
}

function loadFile(path: string) {
  return path;
}

class GuanLiYuan {
  ziDuan = 1;
}

interface YongHuXinXi {
  id: number;
}

type Person = {
  name: string;
};
