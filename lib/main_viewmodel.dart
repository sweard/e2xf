import 'dart:convert';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'src/rust/api/bridge.dart' as lib;

class MainViewModel extends ChangeNotifier {
  MainViewModel() {
    defaultCfg = lib.getDefaultCfg();
    cfgController.text = defaultCfg;
    updateLog("Application initialized.");
  }

  String? _selectedJsonPath;
  String? _selectedExcelPath;
  String? _selectedXmlFolderPath;
  String defaultCfg = "";
  
 final TextEditingController cfgController = TextEditingController();

  String _log = "";

  String? get selectedJsonPath => _selectedJsonPath;
  String? get selectedExcelPath => _selectedExcelPath;
  String? get selectedXmlFolderPath => _selectedXmlFolderPath;
  String get log => _log;
  // 选择文件夹的方法
  Future<void> selectFolder() async {
    String? folderPath = await FilePicker.platform.getDirectoryPath();
    if (folderPath != null) {
      _selectedXmlFolderPath = folderPath;
      notifyListeners();
    }
  }

  // 选择JSON文件的方法
  Future<void> selectJsonFile() async {
    FilePickerResult? result = await FilePicker.platform.pickFiles(
      type: FileType.custom,
      allowedExtensions: ['json'],
    );
    if (result != null) {
      PlatformFile file = result.files.first;
      _selectedJsonPath = file.path;
      notifyListeners();
    }
  }

  // 选择Excel文件的方法
  Future<void> selectExcelFile() async {
    FilePickerResult? result = await FilePicker.platform.pickFiles(
      type: FileType.custom,
      allowedExtensions: ['xlsx', 'xls'],
    );
    if (result != null) {
      PlatformFile file = result.files.first;
      _selectedExcelPath = file.path;
      final sheetNames = lib.getSheetNames(filePath: _selectedExcelPath!);
      final json = jsonDecode(defaultCfg);
      final sheetName = json['sheetName'];
      updateLog(sheetName);
      updateLog(sheetNames.toString());
      if (sheetName.toString().isEmpty &&
          sheetNames.isNotEmpty) {
        json['sheetName'] = sheetNames.first;
      }
      defaultCfg = jsonEncode(json);
      cfgController.text = defaultCfg;
      updateLog(defaultCfg);
      notifyListeners();
    }
  }

  Future<void> update() async {
    if (_selectedJsonPath == null ||
        _selectedExcelPath == null ||
        _selectedXmlFolderPath == null) {
      updateLog("Please select all required paths before updating.");
      return; // 确保所有路径都已选择
    }
    final result = await lib.update(
      cfgJson: _selectedJsonPath!,
      excelPath: _selectedExcelPath!,
      xmlDirPath: _selectedXmlFolderPath!,
    );
    updateLog(result);
  }

  void updateLog(String message) {
    _log += '\n$message';
    notifyListeners();
  }
}
